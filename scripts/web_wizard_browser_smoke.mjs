// Opt-in regression for the isolated Rust browser fixture, never production.
import assert from "node:assert/strict";
import {createRequire} from "node:module";
import {mkdtemp,mkdir,rm} from "node:fs/promises";
import {tmpdir} from "node:os";
import {join} from "node:path";
const [origin,modules,mode="local",screenshot]=process.argv.slice(2);
assert.match(origin??"",/^http:\/\/127\.0\.0\.1:\d+$/);
assert.ok(modules,"Pass an explicit Node modules directory");
const {chromium}=createRequire(import.meta.url)(join(modules,"playwright"));
const root=await mkdtemp(join(tmpdir(),"himalaya-settings-browser-"));
const browser=await chromium.launch({channel:"chrome",headless:true});
const page=await browser.newPage({viewport:mode==="synthetic"?{width:390,height:844}:{width:1280,height:1000}});
const errors=[],foreign=[],publicBodies=[];
page.on("pageerror",e=>errors.push(e.message));
page.on("request",r=>{if(!r.url().startsWith(origin+"/"))foreign.push(r.url());});
page.on("response",async r=>{if(r.url().includes("/api/")){try{publicBodies.push(await r.text());}catch{}}});
const secret="BROWSER_DUMMY_SECRET_MARKER";
async function check(){await page.locator("#destination").check();await page.locator("#test").click();await page.locator("#checked").waitFor({state:"visible"});}
try{
  await page.goto(origin+"/#"+"a".repeat(64));
  await page.locator("#email").waitFor();
  await page.waitForFunction(()=>document.querySelector("#backend option[value=maildir]"));
  assert.equal(await page.evaluate(()=>location.hash),"");
  if(screenshot)await page.screenshot({path:screenshot,fullPage:true});
  if(mode==="local"){
    for(const name of ["mail-one","mail-two"]){
      const path=join(root,name);await mkdir(path);
      await page.locator("#backend").selectOption("maildir");await page.locator("#name").fill(name);await page.locator("#root").fill(path);
      await check();
      await page.locator("#root").fill(path+"-changed");assert.equal(await page.locator("#save").isDisabled(),true);assert.equal(await page.locator("#destination").isChecked(),false);
      await page.locator("#root").fill(path);await check();await page.locator("#save").click();await page.locator("#success").waitFor({state:"visible"});
      if(name==="mail-one")await page.locator("#another").click();
    }
  }else{
    // Requires scripts/web_settings_fixture.sh as the test-only worker.
    await page.locator("#email").fill("dummy@example.test");await page.locator("#discover").click();await page.locator("#discovered").waitFor({state:"visible"});
    assert.equal(await page.locator("#service option").count(),4);
    await page.locator("#service").selectOption("2");await page.locator("#name").fill("synthetic");
    await page.locator("#primary-credential details").click();await page.locator("#primary-source").selectOption("raw");
    await page.locator("#primary-value").fill(secret);await page.locator("#primary-confirm").fill(secret);await page.locator("#primary-approved").check();
    await page.locator("#primary-source").selectOption("custom");assert.equal(await page.locator("#primary-approved").isChecked(),false);
    await page.locator("#primary-entry").fill("printf fixture");await page.locator("#primary-approved").check();await page.locator("#primary-entry").fill("printf changed");assert.equal(await page.locator("#primary-approved").isChecked(),false);
    await page.locator("#primary-source").selectOption("raw");await page.locator("#primary-value").fill(secret);await page.locator("#primary-confirm").fill(secret);await page.locator("#primary-approved").check();await check();
    assert.ok(!(await page.locator("#preview").textContent()).includes(secret));assert.ok(publicBodies.every(body=>!body.includes(secret)));
    assert.equal(await page.locator("#primary-value").inputValue(),"");await page.locator("#save").click();await page.locator("#success").waitFor({state:"visible"});
  }
  assert.equal(await page.evaluate(()=>localStorage.length+sessionStorage.length),0);
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
  assert.deepEqual(errors,[]);assert.deepEqual(foreign,[]);
  await page.locator("#finish").click();await page.getByText("設定を終了しました。このタブを閉じてください。",{exact:true}).waitFor();
  console.log(JSON.stringify({mode,passed:true,pageErrors:errors.length,foreignPageRequests:foreign.length}));
}finally{await browser.close();await rm(root,{recursive:true,force:true});}
