"use strict";
(() => {
  const $ = id => document.getElementById(id);
  const labels = {imap:"IMAP / SMTP",jmap:"JMAP",gmail:"Gmail API",msgraph:"Microsoft Graph",maildir:"Maildir",m2dir:"M2dir",mbox:"Mbox"};
  const methods = [["plain","PLAIN（パスワード）"],["login","LOGIN（パスワード）"],["scram-sha-256","SCRAM-SHA-256"],["oauthbearer","OAUTHBEARER（トークン）"],["xoauth2","XOAUTH2（トークン）"],["anonymous","ANONYMOUS（認証なし）"]];
  let token=location.hash.slice(1);
  history.replaceState(null,"",location.pathname);
  let revision=0,epoch=0,generation="",ticket="",timer;
  let pending=Promise.resolve(),running=false,saving=false,ended=false,recovery=false;
  let accounts=[],choices=[],nameEdited=false,loginEdited=false,legacyMetadata=null;
  const credentials={};
  function options(select,items,selected) {
    select.replaceChildren(...items.map(([value,text])=>{const o=document.createElement("option");o.value=value;o.textContent=text;return o;}));
    if(items.some(([value])=>value===selected))select.value=selected;
  }
  for(const prefix of ["primary","secondary"]) {
    const block=$("credential-template").content.cloneNode(true),c={};
    for(const node of block.querySelectorAll("[data-role]")){c[node.dataset.role]=node;node.id=prefix+"-"+node.dataset.role;}
    for(const label of block.querySelectorAll("[data-label]"))label.htmlFor=prefix+"-"+label.dataset.label;
    $(prefix+"-credential").append(block);credentials[prefix]=c;
  }
  function secret(c){const source=c.source.value,value=["new","raw"].includes(source);return {source,value:value?c.value.value:"",confirm:source==="raw"?c.confirm.value:"",entry:value?"":c.entry.value,approved:["raw","custom"].includes(source)&&c.approved.checked};}
  function clearSecrets(){for(const c of Object.values(credentials)){c.value.value="";c.confirm.value="";}}
  function scrub(d){for(const c of [d.receive,d.smtp]){c.credential.value="";c.credential.confirm="";}}
  function message(text="",error=false){$("feedback").textContent=text;$("feedback").hidden=!text;$("feedback").classList.toggle("error",error);}
  function clearErrors(){for(const node of document.querySelectorAll(".field-error")){node.textContent="";node.hidden=true;}}
  function invalid(field,error){const node=$(field+"-error");if(node){node.textContent=error;node.hidden=false;}if(["receive","smtp","server","root","user-id"].includes(field))$("connection-details").open=true;message(error,true);}
  async function api(path,body,limit=12000){
    const controller=new AbortController(),timeout=setTimeout(()=>controller.abort(),limit);
    try{
      const response=await fetch(path,{method:body===undefined?"GET":"POST",headers:{"X-Himalaya-Setup-Token":token,...(body===undefined?{}:{"Content-Type":"application/json"})},body:body===undefined?undefined:JSON.stringify(body),credentials:"omit",cache:"no-store",redirect:"error",signal:controller.signal});
      const data=await response.json();if(!response.ok)throw new Error(data.error||"処理に失敗しました。設定を確認してください。");return data;
    }finally{clearTimeout(timeout);}
  }
  function buttons(){
    $("all-fields").disabled=ended||saving||recovery;
    for(const id of ["test","discover"])$(id).disabled=ended||saving||running||recovery;
    $("save").disabled=ended||saving||running||!ticket||recovery;
    $("finish").disabled=ended||saving;
    for(const id of ["add-account","another"])$(id).disabled=ended||saving||recovery;
    $("cancel-operation").hidden=!running;
    $("state").textContent=recovery?"復旧確認が必要":saving?"保存中…":running?"確認中…":ticket?"接続確認済み":"未確認";
    $("action-hint").textContent=recovery?"再実行せず復旧情報を確認してください":saving?"Keychainと設定を保存しています…":running?"入力はそのまま修正できます":ticket?"確認した内容を保存できます":"まだ設定は保存していません";
  }
  function cancel(){
    epoch++;clearTimeout(timer);running=false;
    const old=generation;generation="";ticket="";$("checked").hidden=true;$("preview").textContent="";
    if(old)pending=pending.catch(()=>{}).then(()=>api("/api/settings/cancel",{generation:old})).catch(()=>{});
    buttons();
  }
  function setMethods(){const old=$("mechanism").value;options($("mechanism"),$("backend").value==="jmap"?[["basic","Basic（パスワード）"],["bearer","Bearer（トークン）"],["header","Authorizationヘッダー全体"]]:methods,old);}
  function updateCredential(c,mechanism){
    const source=c.source.value,value=["new","raw"].includes(source),oauth=["oauthbearer","xoauth2","bearer","header","api"].includes(mechanism);
    c.value.hidden=!value;c.value.previousElementSibling.hidden=!value;c["value-hint"].hidden=!value;
    c["entry-row"].hidden=value;c["confirm-row"].hidden=source!=="raw";c["approval-row"].hidden=!["custom","raw"].includes(source);
    c["risk-text"].textContent=source==="raw"?"秘密が設定ファイルに平文保存されることを理解し、許可します。":"このコマンドが自分のMacの権限で実行されることを理解し、許可します。";
    c.value.previousElementSibling.textContent=oauth?(mechanism==="header"?"Authorizationヘッダー（例：Bearer …）":"アクセストークン"):"パスワード / アプリパスワード";
    c["value-hint"].textContent=source==="raw"?"非推奨：秘密そのものを設定ファイルに保存します。":"接続確認後、「保存する」でこのMacのKeychainへ登録します。";
    c["entry-hint"].textContent=source==="custom"?"秘密を標準出力の先頭行に返すコマンド。秘密そのものをコマンドに書かないでください。":source==="security"?"MacのKeychainに保存済みのアカウント名。":"選択した外部ツールに登録済みの項目名。ツール側の設定を先に行ってください。";
    c.entry.previousElementSibling.textContent=source==="custom"?"認証情報を取得するコマンド":"保存済み項目名 / 管理ツールのアカウント名";
    c["source-hint"].textContent=oauth?"OAuthの発行・更新は外部ツールで行います。Keychainへの新規保存は入力した値を固定で保存します。":"既存の秘密は読み戻しません。新規項目は既存項目を上書きしません。";
  }
  function update(){
    const backend=$("backend").value,local=["maildir","m2dir","mbox"].includes(backend),imap=backend==="imap",jmap=backend==="jmap",apiBackend=["gmail","msgraph"].includes(backend);
    if(!imap){$("legacy-enabled").checked=false;$("legacy-approved").checked=false;}
    for(const [id,show]of [["imap-settings",imap],["jmap-settings",jmap],["api-settings",apiBackend],["local-settings",local],["authentication",!local]])$(id).hidden=!show;
    $("login-row").hidden=apiBackend||(jmap&&$("mechanism").value!=="basic")||(imap&&$("mechanism").value==="anonymous");
    $("mechanism-row").hidden=apiBackend;$("primary-credential").hidden=imap&&$("mechanism").value==="anonymous";
    $("smtp-server-settings").hidden=!$("smtp-enabled").checked;$("smtp-auth-toggle").hidden=!imap||!$("smtp-enabled").checked;
    $("smtp-authentication").hidden=!imap||!$("smtp-enabled").checked||$("smtp-shared").checked;$("secondary-credential").hidden=$("smtp-mechanism").value==="anonymous";
    $("legacy-details").hidden=!imap||!legacyMetadata?.available;$("legacy-options").hidden=!$("legacy-enabled").checked;
    updateCredential(credentials.primary,apiBackend?"api":$("mechanism").value);updateCredential(credentials.secondary,$("smtp-mechanism").value);
    const endpoint=(prefix,label)=>label+"  "+($(prefix+"-host").value||"未入力")+":"+$(prefix+"-port").value+" / "+($(prefix+"-encryption").selectedOptions[0]?.textContent||"暗号化方式を選択してください");
    $("endpoints").textContent=imap?endpoint("receive","IMAP")+($("smtp-enabled").checked?"\n"+endpoint("smtp","SMTP"):"\nSMTP：設定しない")+($("legacy-enabled").checked?"\n注意：旧TLS 1.0・証明書固定の互換モード":""):jmap?"JMAP  "+($("server").value||"URL未入力"):apiBackend?(backend==="gmail"?"Gmail API  https://gmail.googleapis.com":"Microsoft Graph  https://graph.microsoft.com")+"\nユーザーID："+$("user-id").value:(labels[backend]||backend)+"  "+($("root").value||"保存先未入力");buttons();
  }
  function changed(event){
    if(saving||ended||recovery)return;
    revision++;cancel();clearErrors();message();
    if(event?.target.id!=="destination")$("destination").checked=false;
    if(["backend","receive-host","receive-port","receive-encryption","smtp-host","smtp-port","smtp-encryption","smtp-enabled","legacy-enabled","legacy-imap-pin","legacy-smtp-pin"].includes(event?.target.id))$("legacy-approved").checked=false;
    if(event?.target.id==="name")nameEdited=true;if(event?.target.id==="login")loginEdited=true;
    for(const c of Object.values(credentials)){
      if(event?.target===c.source||event?.target===c.entry)c.approved.checked=false;
      if(event?.target===c.source){c.value.value="";c.confirm.value="";}
    }
    if(event?.target.id==="email"){
      if(!loginEdited){$("login").value=$("email").value.trim();$("smtp-login").value=$("email").value.trim();}
      if(!nameEdited){const base=($("email").value.split("@")[1]||"").split(".")[0].replace(/[^a-zA-Z0-9_-]/g,"").slice(0,48);let name=base,n=2;while(name&&accounts.includes(name))name=base+"-"+n++;$("name").value=name;}
    }
    if(event?.target.id==="backend")setMethods();update();
  }
  function draft(){
    const connection=(prefix,c,primary=false)=>({host:$(prefix+"-host").value.trim(),port:Number($(prefix+"-port").value),encryption:$(prefix+"-encryption").value,login:$(primary?"login":"smtp-login").value.trim(),mechanism:$(primary?"mechanism":"smtp-mechanism").value,credential:secret(c)});
    return {name:$("name").value.trim(),email:$("email").value.trim(),backend:$("backend").value,receive:connection("receive",credentials.primary,true),smtp:connection("smtp",credentials.secondary),smtp_enabled:$("smtp-enabled").checked,smtp_shared:$("smtp-shared").checked,server:$("server").value.trim(),user_id:$("user-id").value.trim(),root:$("root").value.trim(),destination_confirmed:$("destination").checked,legacy_mprg:$("legacy-enabled").checked?{imap_sha256:$("legacy-imap-pin").value.trim(),smtp_sha256:$("legacy-smtp-pin").value.trim(),approved:$("legacy-approved").checked}:null};
  }
  function applyChoice(index){
    const choice=choices[index];if(!choice)return;changed();$("backend").value=choice.backend;setMethods();
    for(const [prefix,value]of [["receive",choice.receive],["smtp",choice.smtp]])if(value)for(const key of ["host","port","encryption"])$(prefix+"-"+key).value=value[key];
    if(choice.backend==="imap"){$("smtp-enabled").checked=!!choice.smtp;$("login").value=choice.receive?.login||$("email").value.trim();$("smtp-login").value=choice.smtp?.login||$("login").value;}
    if(choice.server)$("server").value=choice.server;update();message("設定を取得しました。接続先と認証情報を確認してください。");
  }
  async function run(action,payload){
    if(running||saving||ended||recovery)return;cancel();clearErrors();message();const current=epoch,version=String(revision);running=true;buttons();
    try{
      await pending;if(ended||current!==epoch)return;
      const request=api("/api/settings/"+action,{...payload,revision:version});
      pending=request.then(data=>{
        if((ended||current!==epoch)&&data.generation)return api("/api/settings/cancel",{generation:data.generation});
      }).catch(()=>{});
      if(payload.draft){clearSecrets();scrub(payload.draft);}
      const data=await request;
      if(ended||current!==epoch)return;
      if(data.state==="invalid"){running=false;invalid(data.field,data.error);buttons();return;}
      generation=data.generation;
      const poll=async()=>{
        if(ended||current!==epoch)return;
        try{
          const result=await api("/api/settings/poll",{generation});if(ended||current!==epoch)return;
          if(result.state==="working"){timer=setTimeout(poll,350);return;}running=false;
          if(result.revision&&result.revision!==String(revision)){cancel();return;}
          if(result.state==="ready"){ticket=result.ticket;$("preview").textContent=result.preview;$("checked").hidden=false;message();}
          else if(result.state==="discovered"){
            choices=result.result.choices||[];
            if(!choices.length)invalid("email",result.result.error||"自動取得できませんでした。接続設定を手入力してください。");
            else{options($("service"),choices.map((choice,i)=>[String(i),choice.label||labels[choice.backend]]));$("discovered").hidden=choices.length<2;applyChoice(0);}
          }else invalid(result.field||"connection",result.error||"確認を完了できませんでした。設定を修正して再試行してください。");buttons();
        }catch(error){if(current===epoch&&!ended){cancel();message(error.message,true);}}
      };await poll();
    }catch(error){if(current===epoch&&!ended){cancel();message(error.message,true);}}
    finally{if(payload.draft)scrub(payload.draft);}
  }
  function renderAccounts(){$("accounts").replaceChildren(...(accounts.length?accounts:["登録はありません"]).map(name=>{const li=document.createElement("li");li.textContent=name;return li;}));}
  function reset(){cancel();revision++;clearSecrets();$("settings-form").reset();clearErrors();message();nameEdited=false;loginEdited=false;choices=[];$("discovered").hidden=true;$("settings-form").hidden=false;$("success").hidden=true;setMethods();update();$("email").focus();}
  $("all-fields").addEventListener("input",changed);
  $("all-fields").addEventListener("change",event=>{if(event.target.tagName==="SELECT")changed(event);});
  $("service").addEventListener("change",()=>applyChoice(Number($("service").value)));
  $("legacy-preset").addEventListener("click",()=>{
    if(!legacyMetadata?.available||saving||ended||recovery)return;changed();$("backend").value="imap";setMethods();
    $("receive-host").value=legacyMetadata.host;$("receive-port").value="143";$("receive-encryption").value="starttls";
    $("smtp-host").value=legacyMetadata.host;$("smtp-port").value="587";$("smtp-encryption").value="starttls";$("smtp-enabled").checked=true;$("smtp-shared").checked=true;
    $("login").value=$("email").value.trim().split("@")[0];$("smtp-login").value=$("login").value;loginEdited=true;
    $("legacy-imap-pin").value=legacyMetadata.observed_sha256;$("legacy-smtp-pin").value=legacyMetadata.observed_sha256;$("legacy-enabled").checked=true;$("legacy-approved").checked=false;
    $("legacy-pin-info").textContent=legacyMetadata.observed_date+"の認証情報なしの調査で観測した証明書です。独立した本人確認済み証明書ではありません。変更時は自動更新しません。";update();
  });
  $("discover").addEventListener("click",()=>run("discover",{email:$("email").value.trim()}));
  $("settings-form").addEventListener("submit",event=>{event.preventDefault();if(!$("destination").checked){invalid("destination","接続先を確認し、確認欄にチェックしてください。");return;}run("check",{draft:draft()});});
  $("cancel-operation").addEventListener("click",()=>{cancel();clearSecrets();message("確認を中止しました。入力を修正して再確認できます。");});
  $("save").addEventListener("click",async()=>{
    if(!ticket||running||saving||ended||recovery)return;saving=true;buttons();const commit={ticket,revision:String(revision)};ticket="";
    const complete=data=>{
      if(data.state==="recovery_required"){recovery=true;$("checked").hidden=true;message(data.error+" 再実行せず、起動元のターミナルの復旧情報と設定ファイルを確認してください。",true);}
      else if(data.state!=="saved"){ $("checked").hidden=true;message((data.error||"保存できませんでした。")+" 再確認する場合は認証情報をもう一度入力してください。",true); }
      else{accounts.push(data.saved);renderAccounts();$("saved-name").textContent=data.saved;$("settings-form").hidden=true;$("success").hidden=false;clearSecrets();$("preview").textContent="";}
      saving=false;buttons();
    };
    try{complete(await api("/api/settings/save",commit,60000));}
    catch(_){
      message("保存結果を確認中です。再実行せず、そのままお待ちください。",true);
      const reconcile=async()=>{
        try{complete(await api("/api/settings/result",commit));}
        catch(_){message("保存結果をまだ確定できません。再実行やタブの再読込をせず、起動元のターミナルと設定ファイルを確認してください。",true);timer=setTimeout(reconcile,1500);}
      };await reconcile();
    }
  });
  $("another").addEventListener("click",reset);$("add-account").addEventListener("click",reset);
  $("finish").addEventListener("click",async()=>{if(ended||saving)return;cancel();clearSecrets();try{await pending;await api("/api/finish",{});ended=true;token="";buttons();$("session-error").textContent="設定を終了しました。このタブを閉じてください。";$("session-error").hidden=false;}catch(_){message("終了を確認できませんでした。起動元のターミナルでCtrl-Cを押して終了してください。",true);}});
  async function initialize(){
    if(location.protocol==="file:"){$("file-warning").hidden=false;ended=true;buttons();return;}
    if(!/^[0-9a-f]{64}$/.test(token)){ended=true;buttons();$("session-error").textContent="設定セッションがありません。Himalaya設定.command から開き直してください。";$("session-error").hidden=false;return;}
    try{const data=await api("/api/status");accounts=data.accounts||[];legacyMetadata=data.legacy_mprg||null;renderAccounts();options($("backend"),data.backends.map(backend=>[backend,labels[backend]||backend]));for(const c of Object.values(credentials))options(c.source,data.sources.map(source=>[source.id,source.label+(source.installed?"":"（未導入）")]),"new");options($("smtp-mechanism"),methods);setMethods();update();$("email").focus();}
    catch(error){ended=true;buttons();$("session-error").textContent=error.message+" 起動し直してください。";$("session-error").hidden=false;}
  }
  initialize();
})();
