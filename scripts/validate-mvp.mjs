// Run only against the isolated com.local.musicarchive.mvpvalidation dev app.
import assert from 'node:assert/strict';
import fs from 'node:fs';
const targets = await (await fetch('http://127.0.0.1:9224/json')).json();
const target = targets.find(t => t.url === 'http://localhost:1422/');
assert(target, 'Start the isolated validation app on port 1422.');
const ws = new WebSocket(target.webSocketDebuggerUrl);
await new Promise(resolve => ws.addEventListener('open', resolve, {once:true}));
let serial=0;
const pending=new Map();
ws.onmessage=e=>{const message=JSON.parse(e.data);if(message.id){pending.get(message.id)?.(message);pending.delete(message.id);}};
function call(method,params={}){return new Promise((resolve,reject)=>{const id=++serial;const timer=setTimeout(()=>{pending.delete(id);reject(new Error(`Timed out: ${method}`));},120000);pending.set(id,m=>{clearTimeout(timer);m.error?reject(m.error):resolve(m.result);});ws.send(JSON.stringify({id,method,params}));});}
async function evaluate(expression){const r=await call('Runtime.evaluate',{expression,awaitPromise:true,returnByValue:true});if(r.exceptionDetails)throw new Error(JSON.stringify(r.exceptionDetails));return r.result.value;}
async function wait(expression){for(let i=0;i<600;i++){if(await evaluate(`Boolean(${expression})`))return;await new Promise(r=>setTimeout(r,100));}throw new Error(`Wait: ${expression}\n${await evaluate('document.body.innerText')}`);}
const invoke=(command,args={})=>evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})`);
const click=selector=>evaluate(`document.querySelector(${JSON.stringify(selector)}).click()`);
async function nav(label){await evaluate(`Array.from(document.querySelectorAll('.nav-item')).find(b=>b.textContent===${JSON.stringify(label)}).click()`);}
async function fill(selector,value){await evaluate(`(()=>{const el=document.querySelector(${JSON.stringify(selector)});Object.getOwnPropertyDescriptor(el.tagName==='TEXTAREA'?HTMLTextAreaElement.prototype:HTMLInputElement.prototype,'value').set.call(el,${JSON.stringify(value)});el.dispatchEvent(new Event('input',{bubbles:true}));})()`);}
const button=text=>evaluate(`Array.from(document.querySelectorAll('button')).find(b=>b.textContent===${JSON.stringify(text)}&&!b.closest('[hidden]')).click()`);
async function screenshot(name){const r=await call('Page.captureScreenshot',{format:'png',captureBeyondViewport:true});fs.writeFileSync(`dist/${name}.png`,Buffer.from(r.data,'base64'));}
try {
  await wait('document.querySelectorAll(".nav-item").length===6');
  const tracks=await invoke('library_tracks');assert(tracks.length>1);
  const trackId=tracks[0].id;
  if(process.argv.includes('--connect')){
    await nav('Settings');
    console.log('Waiting for Spotify permission in your browser...');
    console.log('Spotify connection:', await invoke('spotify_connect'));
  } else if(process.argv.includes('--spotify')){
    try { console.log('Spotify playlists:',await invoke('spotify_sync_playlists')); }
    catch(e){console.log('REAL SPOTIFY VALIDATION BLOCKED:',e.message);process.exitCode=2;}
  } else if(process.argv.includes('--restart')){
    const saved=JSON.parse(fs.readFileSync('dist/mvp-validation-state.json','utf8'));
    assert.equal(await invoke('track_note',{trackId:saved.trackId}),saved.note);
    const d=await invoke('digging_detail',{diggingId:saved.diggingId});assert.equal(d.session.end_date,'2026-09-16');assert.equal(d.tracks.length,1);
    const p=await invoke('playlist_detail',{playlistId:saved.playlistId});assert.equal(p.personal_description,'Personal playlist description');assert(p.tags.some(t=>t.name==='MVP QA'));
    console.log('PASS: actual app restart preserved note, ended digging, membership, playlist description and tags.');
  } else {
    await nav('Library');await wait('document.querySelector(".library-track-button")');await click('.library-track-button');await wait('document.querySelector("#track-note")');
    const note='MVP validation 메모\nSecond line';await fill('#track-note',note);await button('Save note');await wait('document.body.innerText.includes("Note saved.")');assert.equal(await invoke('track_note',{trackId}),note);
    await nav('Calendar');await wait('document.querySelectorAll(".calendar-grid button").length===30');
    assert(await evaluate('document.body.innerText.includes("Asia/Seoul")'));
    await screenshot('mvp-calendar');
    await nav('Digging');await wait('!document.body.innerText.includes("Loading digging")');await button('New digging');await fill('.archive-form input:not([type])','MVP QA Digging');await fill('.archive-form input[type=date]','2026-09-01');await button('Save digging');
    await wait('document.querySelector(".track-picker button")');await click('.track-picker button');await wait('document.querySelectorAll(".archive-track").length===1');
    const sessions=await invoke('digging_sessions');const d=sessions.find(d=>d.name==='MVP QA Digging');assert(d);
    await button('Edit / End digging');await fill('.archive-form input[type=date]:nth-of-type(1)','2026-09-01');
    await evaluate(`(()=>{const inputs=document.querySelectorAll('.archive-form input[type=date]');const el=inputs[1];Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(el,'2026-09-16');el.dispatchEvent(new Event('input',{bubbles:true}));})()`);
    await button('Save digging');await wait('!document.querySelector(".archive-form")');assert.equal((await invoke('digging_detail',{diggingId:d.id})).session.end_date,'2026-09-16');
    await screenshot('mvp-digging');
    await nav('Playlists');await wait('document.querySelector(".collection-card")');await evaluate(`Array.from(document.querySelectorAll('.collection-card')).find(b=>b.textContent.includes('MVP QA Playlist')).click()`);await wait('document.querySelector("#playlist-description")');
    await fill('#playlist-description','Personal playlist description');await button('Save notes');await wait('document.body.innerText.includes("Notes saved.")');
    const p=(await invoke('archived_playlists')).find(p=>p.name==='MVP QA Playlist');assert(p);
    const tag=await invoke('create_tag',{name:'MVP QA',category:'free'});await invoke('set_playlist_tag',{playlistId:p.id,tagId:tag.id,assigned:true});await invoke('set_playlist_tag',{playlistId:p.id,tagId:tag.id,assigned:true});
    const detail=await invoke('playlist_detail',{playlistId:p.id});assert.equal(detail.tags.filter(t=>t.id===tag.id).length,1);assert.equal(detail.tracks.length,2);
    await screenshot('mvp-playlist');
    await call('Network.enable');await call('Network.emulateNetworkConditions',{offline:true,latency:0,downloadThroughput:0,uploadThroughput:0});
    await nav('Home');await wait('document.querySelector(".home-summary")');await screenshot('mvp-home');assert(await invoke('track_note',{trackId}));
    await call('Emulation.setDeviceMetricsOverride',{width:800,height:650,deviceScaleFactor:1,mobile:false});await nav('Calendar');await wait('document.querySelectorAll(".calendar-grid button").length===30');assert(await evaluate('document.documentElement.scrollWidth<=800'));await screenshot('mvp-calendar-800');
    await call('Emulation.clearDeviceMetricsOverride');
    fs.writeFileSync('dist/mvp-validation-state.json',JSON.stringify({trackId,note,diggingId:d.id,playlistId:p.id}));
    console.log('PASS: real UI note save, local calendar, digging create/add/end, playlist description, duplicate-safe playlist tags, offline Home, 800px Calendar.');
  }
} finally {
  await call('Network.emulateNetworkConditions',{offline:false,latency:0,downloadThroughput:-1,uploadThroughput:-1}).catch(()=>{});
  ws.close();
}
