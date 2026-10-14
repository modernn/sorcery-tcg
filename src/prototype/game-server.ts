import { createServer, type IncomingMessage, type Server, type ServerResponse } from 'node:http';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { createSyntheticDemoManifest } from '../commands/run-game-demo.ts';
import { canonicalJson, type JsonValue } from '../authority/canonical-json.ts';
import { GAME_CHECKPOINT_MAX_BYTES, parseGameCheckpoint } from '../engine/checkpoint.ts';
import {
  createGameManifest,
  type GameCardDefinition,
  type GameLegalAction,
  type GameObservation,
  type GameSeat,
  type GameManifest,
  type GameSession,
} from '../engine/game.ts';
import { identityHash } from '../authority/hash.ts';
import { RustSessionRpcError, RustSessionClient, type RustLegalAction, type Sha256Hash } from '../engine/rust-engine.ts';

const HOST = '127.0.0.1';
const DEFAULT_PORT = 4174;
const MAX_BODY_BYTES = 65_536;
const MAX_OPPONENT_ACTIONS = 500;

type GameOpponent = 'manual' | 'south';

export type GamePreset = Readonly<{
  cardNames?: Readonly<Record<string, string>>;
  id: string;
  label: string;
  manifest: GameManifest;
}>;

const CELLS = Array.from({ length: 4 }, (_, row) =>
  Array.from({ length: 5 }, (_, column) =>
    `<div class="cell" data-cell="${String.fromCharCode(65 + column)}${row + 1}"></div>`).join('')).join('');

const PAGE = String.raw`<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="initial-scale=1,width=device-width">
  <title>Sorcery Simulator — Playable Core</title>
  <style>
    :root{color-scheme:dark;--bg:#0d171d;--panel:#14262f;--line:#45606b;--ink:#e1e9e6;--muted:#91a7a9;--north:#3984a1;--south:#bd5949;--site:#718458;--focus:#ffc96b;font-family:Bahnschrift,"Segoe UI",sans-serif;background:var(--bg);color:var(--ink)}*{box-sizing:border-box}body{margin:0;background:radial-gradient(circle at 45% 10%,#294853,transparent 38%),var(--bg);min-height:100vh}button,input,select{font:inherit}button{background:#243d48;border:1px solid #5b7b87;border-radius:.35rem;color:var(--ink);cursor:pointer;padding:.58rem .72rem}button:hover{background:#315362}button:focus-visible,input:focus-visible,select:focus-visible,summary:focus-visible,.action:focus,.action.is-focused{outline:3px solid var(--focus);outline-offset:2px}button:disabled{opacity:.45;cursor:not-allowed}header,.toolbar{display:flex;gap:.7rem;align-items:center;flex-wrap:wrap;padding:1rem clamp(1rem,4vw,3rem);border-bottom:1px solid var(--line);background:#0c171ddd}header{justify-content:space-between}h1{margin:0;font-size:clamp(1.35rem,3vw,2.15rem);letter-spacing:.06em;text-transform:uppercase}.eyebrow,.badge,label{font-size:.72rem;letter-spacing:.12em;text-transform:uppercase}.eyebrow,label{color:var(--muted)}.badge{border:1px solid #d29b55;border-radius:999px;color:#ffd89c;padding:.35rem .55rem}.toolbar label{display:grid;gap:.25rem}.toolbar input,.toolbar select{background:#091318;border:1px solid #52707b;border-radius:.3rem;color:var(--ink);padding:.55rem}.toolbar input{width:10rem}.toolbar select{max-width:22rem}.seat-switch{display:flex;margin-left:auto}.seat-switch button{border-radius:0}.seat-switch button:first-child{border-radius:.35rem 0 0 .35rem}.seat-switch button:last-child{border-radius:0 .35rem .35rem 0}.seat-switch [aria-pressed=true]{background:#2f7189}.layout{display:grid;grid-template-columns:minmax(0,1fr) minmax(19rem,26rem);gap:1rem;padding:1rem clamp(1rem,4vw,3rem) 2rem;max-width:1600px;margin:auto}.table{position:relative;min-height:43rem;border:1px solid var(--line);border-radius:.65rem;background:#142a33;box-shadow:inset 0 0 5rem #0008}.realm{position:absolute;inset:19% 10%;display:grid;grid-template-columns:repeat(5,1fr);grid-template-rows:repeat(4,minmax(0,1fr));gap:.4rem}.cell{position:relative;border:1px solid #78949b;background:#0d2028;display:grid;place-items:center;align-content:safe center;gap:.2rem;min-width:0;overflow:auto}.cell:after{content:attr(data-cell);position:absolute;right:.2rem;bottom:.15rem;color:#71888e;font:600 .62rem "Cascadia Mono",monospace}.piece{display:grid;gap:.15rem;text-align:center;max-width:95%;font:700 .68rem "Cascadia Mono",monospace}.piece small,.card small{display:block;font-weight:500}.site{background:#34432c;border:1px solid #879b68;border-radius:.25rem;padding:.35rem}.unit{background:#263f4a;border:1px solid #73a3b4;border-radius:.25rem;padding:.2rem}.artifact{background:#4a3726;border:1px solid #b78b58;border-radius:.25rem;padding:.2rem}.aura{background:#45365c;border:1px solid #a98bca;border-radius:.25rem;padding:.2rem}.marker{border:1px dashed #7f98a0;border-radius:.2rem;color:#aababc;font-size:.56rem;padding:.12rem}.effect{border-color:#8aa0ca;color:#bdc9e4}.avatar{border:2px solid var(--north);border-radius:999px;padding:.2rem .35rem}.avatar.south{border-color:var(--south)}.player{position:absolute;left:1rem;right:1rem;display:grid;grid-template-columns:auto 1fr;gap:.8rem;padding:.65rem .8rem;background:#09151bdc;border:1px solid var(--line);border-radius:.4rem;z-index:2}.player.north{bottom:1rem;border-left:4px solid var(--north)}.player.south{top:1rem;border-left:4px solid var(--south)}.player h2{font-size:.8rem;text-transform:uppercase;margin:0}.player p{color:var(--muted);font:.72rem/1.4 "Cascadia Mono",monospace;margin:.2rem 0}.hand{display:flex;gap:.3rem;flex-wrap:wrap;justify-content:flex-end}.card{border:1px solid #78949b;background:#1c343e;border-radius:.2rem;padding:.28rem;max-width:10rem;overflow:hidden;text-overflow:ellipsis;font:.64rem "Cascadia Mono",monospace}.sidebar{display:grid;gap:1rem;align-content:start}.panel{border:1px solid var(--line);border-radius:.55rem;background:var(--panel);overflow:hidden}.panel h2{font-size:.76rem;letter-spacing:.12em;text-transform:uppercase;margin:0;padding:.7rem .8rem;border-bottom:1px solid var(--line)}.panel-body{padding:.75rem}.status{display:grid;grid-template-columns:repeat(2,1fr);gap:.45rem}.datum{background:#0b1920;border:1px solid #314a54;border-radius:.3rem;padding:.45rem;min-width:0}.datum b{display:block;color:var(--muted);font-size:.62rem;text-transform:uppercase}.datum span{display:block;overflow:hidden;text-overflow:ellipsis;font:700 .72rem "Cascadia Mono",monospace;margin-top:.18rem}.actions{display:grid;gap:.45rem;max-height:20rem;overflow:auto}.action-list{display:grid;gap:.45rem;padding-top:.55rem}.action{text-align:left;border-left:3px solid var(--north)}details{border-top:1px solid var(--line);margin-top:.55rem;padding-top:.55rem}details:not([open])>:not(summary){display:none}summary{cursor:pointer;color:#c6d3d2}.receipt{white-space:pre-wrap;word-break:break-word;max-height:13rem;overflow:auto;margin:0;font:.7rem/1.45 "Cascadia Mono",monospace;color:#bed0ce}.ok{border-left:3px solid var(--site);padding:.5rem}.error{border-left:3px solid var(--south);padding:.5rem}.empty{color:var(--muted);font-size:.82rem}.footnote{color:#81979a;font-size:.72rem;line-height:1.45}.replay-nav{display:flex;gap:.45rem;flex-wrap:wrap;margin-bottom:.55rem}.hidden{display:none}@media(max-width:900px){.layout{grid-template-columns:1fr}.table{min-height:38rem}.realm{inset:22% 4%}.seat-switch{margin-left:0}}@media(prefers-reduced-motion:reduce){*{transition:none!important}}
  </style>
</head>
<body>
  <header><div><div class="eyebrow">Authoritative rules checkpoint</div><h1>Sorcery Playable Core</h1></div><span class="badge" id="mode">Unranked · partial rules</span></header>
  <form class="toolbar" id="reset-form"><label>Starter matchup<select id="preset" aria-label="Starter matchup"></select></label><label>Opponent<select id="opponent" aria-label="Opponent"><option value="south">South computer</option><option value="manual">Hot seat</option></select></label><label>Seed<input id="seed" inputmode="numeric" min="0" max="4294967295" step="1" value="1" required></label><button>Reset match</button><button type="button" id="save">Save position</button><button type="button" id="resume" disabled>Resume position</button><button type="button" id="replay">Verify replay</button><button type="button" id="stale" disabled>Resubmit stale</button><div class="seat-switch" role="group" aria-label="Observed seat"><button type="button" data-seat="north" aria-pressed="true">North</button><button type="button" data-seat="south" aria-pressed="false">South</button></div></form>
  <main class="layout">
    <section class="table" aria-label="Five by four realm">
      <article class="player south"><div><h2>South</h2><p id="south-stats"></p></div><div class="hand" id="south-hand"></div></article>
      <div class="realm" id="realm">${CELLS}</div>
      <article class="player north"><div><h2>North</h2><p id="north-stats"></p></div><div class="hand" id="north-hand"></div></article>
    </section>
    <aside class="sidebar">
      <section class="panel"><h2>Match state</h2><div class="panel-body status"><div class="datum"><b>Observer</b><span id="observer">north</span></div><div class="datum"><b>Active</b><span id="active">—</span></div><div class="datum"><b>Turn / phase</b><span id="phase">—</span></div><div class="datum"><b>Version</b><span id="version">—</span></div><div class="datum" style="grid-column:1/-1"><b>State hash</b><span id="hash">—</span></div></div></section>
      <section class="panel"><h2>Engine-issued actions</h2><div class="panel-body"><div class="actions" id="actions"><p class="empty">Loading…</p></div><p class="footnote">Large choice sets are grouped; every button remains a fully bound legal action. Number keys 1-9 play the first nine visible actions. Up and Down move among those actions.</p></div></section>
      <section class="panel"><h2>Action result</h2><div class="panel-body"><div id="notice" role="status" aria-live="polite"></div><p id="opponent-summary" class="footnote"></p><details><summary>Technical receipt</summary><pre id="receipt" class="receipt">Starting match…</pre></details></div></section>
      <section class="panel"><h2>Replay steps</h2><div class="panel-body"><div class="replay-nav"><button type="button" id="replay-prev" disabled>Previous step</button><button type="button" id="replay-next" disabled>Next step</button></div><p id="replay-step" role="status" aria-live="polite" class="empty">Verify replay to inspect committed steps.</p><pre id="replay-detail" class="receipt hidden"></pre><p class="footnote">Left and Right arrows move one committed step. Chain status is written in text, not color alone.</p></div></section>
    </aside>
  </main>
  <script>
    var busy=false,sessionReady=false,snapshotCurrent=false,actionsReady=false,outcomeUnknown=false;var seat='north',snapshot,lastCommand,savedPosition,replaySteps=[],replayIndex=0,replayChained=false;var byId=function(id){return document.getElementById(id)},saveKey='sorcery-playable-core-checkpoint-v1';try{savedPosition=JSON.parse(localStorage.getItem(saveKey)||'null')}catch(_error){localStorage.removeItem(saveKey)}if(!savedPosition||typeof savedPosition.checkpoint!=='string'||!['manual','south'].includes(savedPosition.opponent))savedPosition=undefined;
    function record(value){return value!==null&&typeof value==='object'&&!Array.isArray(value)}
    function hash(value){return typeof value==='string'&&/^sha256:[0-9a-f]{64}$/.test(value)}
    function version(value){return Number.isSafeInteger(value)&&value>=0}
    function validRecovery(value){return record(value)&&['player-presentation','opponent-budget','opponent-policy','opponent-step','snapshot','replacement'].includes(value.stage)&&typeof value.message==='string'&&['current','unavailable','untrusted'].includes(value.snapshotStatus)&&typeof value.actionsAvailable==='boolean'&&['ready','blocked','unknown'].includes(value.sessionStatus)&&['known','unknown'].includes(value.metadataStatus)&&(!value.actionsAvailable||value.snapshotStatus==='current')&&(value.sessionStatus==='unknown'?value.snapshotStatus==='untrusted':value.snapshotStatus!=='untrusted'&&value.metadataStatus==='known')}
    function actionReceipt(value,command){return record(value)&&record(command)&&hash(value.actionId)&&value.actionId===command.actionId&&value.seat===command.seat&&['north','south'].includes(value.seat)&&version(value.stateVersion)&&value.stateVersion===command.stateVersion&&value.nextStateVersion===value.stateVersion+1&&version(value.nextStateVersion)&&hash(value.preStateHash)&&hash(value.postStateHash)&&hash(value.receiptId)&&version(value.receiptSequence)&&value.receiptSequence>0&&Array.isArray(value.events)&&value.events.every(function(event){return record(event)&&record(event.cause)&&event.cause.actionId===value.actionId&&event.cause.receiptSequence===value.receiptSequence&&version(event.eventSequence)&&hash(event.eventId)&&typeof event.type==='string'&&'payload' in event})&&Array.isArray(value.randomDraws)&&value.randomDraws.every(function(draw){return record(draw)&&version(draw.drawSequence)&&typeof draw.purpose==='string'&&hash(draw.prePrngStateHash)&&hash(draw.postPrngStateHash)&&'domain' in draw&&'result' in draw})}
    function mutationResult(operation,body,status,command,path){
      if(!record(body)||(body.recovery!==undefined&&!validRecovery(body.recovery)))return false;
      if(body.recovery&&body.recovery.snapshotStatus!=='current'&&['view','stateHash','actions','cardFacts','cardNames'].some(function(key){return body[key]!==undefined}))return false;
      if(status===200){
        if(operation==='action'){
          if(!version(body.opponentActionCount)||!Array.isArray(body.opponentActions)||body.opponentActions.length!==body.opponentActionCount||!body.opponentActions.every(function(item){return record(item)&&typeof item.kind==='string'&&Array.isArray(item.events)&&item.events.every(function(event){return typeof event==='string'})}))return false;
          if(body.accepted===true)return typeof body.playerAction==='string'&&actionReceipt(body.receipt,command);
          var reason=body.reason;return body.accepted===false&&body.receipt===undefined&&body.opponentActionCount===0&&record(reason)&&['stale_version','terminal_state','unknown_action','wrong_seat'].includes(reason.code)&&typeof reason.message==='string'&&version(reason.currentStateVersion)&&hash(reason.currentStateHash);
        }
        var replacement=body.replacement,ack=replacement&&replacement.receipt;
        return record(replacement)&&replacement.status==='confirmed'&&record(ack)&&hash(ack.stateHash)&&version(ack.stateVersion)&&(operation==='reset'?hash(ack.manifestId):hash(ack.expectedSessionHash)&&record(command)&&ack.expectedSessionHash===command.expectedSessionHash);
      }
      if(typeof body.error!=='string'||body.accepted!==undefined)return false;
      if(body.recovery){
        if(body.recovery.sessionStatus==='unknown')return body.recovery.snapshotStatus==='untrusted';
        if(operation==='action')return (status===503&&body.recovery.stage==='player-presentation')||(status===400&&body.recovery.sessionStatus==='blocked'&&['snapshot','opponent-step'].includes(body.recovery.stage));
        return status===400&&body.recovery.stage==='replacement'&&record(body.replacement)&&body.replacement.status==='unchanged'&&body.replacement.receipt===undefined;
      }
      // Trust a legacy refusal only when this exact request could have failed
      // that route's preflight. Broad handler fallbacks can also catch failures
      // after a native acknowledgement, so they stay inconclusive.
      var querySeat=(path.match(/[?&]seat=([^&]*)/)||[])[1]||'',queryOpponent=(path.match(/[?&]opponent=([^&]*)/)||[])[1]||'';
      if(body.replacement!==undefined)return false;
      if(operation==='reset'&&status===400){
        if(body.error==='seed must be an unsigned 32-bit integer')return !Number.isInteger(command.seed)||command.seed<0||command.seed>4294967295;
        if(body.error==='presetId must name an available preset'){
          var presets=snapshot&&snapshot.presets;
          return typeof command.presetId==='string'&&Array.isArray(presets)&&presets.length>0
            &&presets.every(function(item){return record(item)&&typeof item.id==='string'&&item.id.length>0})
            &&presets.some(function(item){return item.id===snapshot.presetId})
            &&!presets.some(function(item){return item.id===command.presetId});
        }
        if(body.error==='opponent must be manual or south')return !['manual','south'].includes(command.opponent);
      }
      if(operation==='resume'){
        if(body.error==='seat and opponent are required'&&status===400)return !['north','south'].includes(querySeat)||!['manual','south'].includes(queryOpponent);
        if(body.error==='south is hidden while controlled by the deterministic opponent'&&status===403)return querySeat==='south'&&queryOpponent==='south';
      }
      if(operation==='action'&&body.error==='south is controlled by the deterministic opponent'&&status===400){
        return command.seat==='south'&&snapshot&&snapshot.opponent==='south';
      }
      return false;
    }
    async function request(path,options){var operation=options&&options.method==='POST'&&(/^\/api\/(action|reset|resume)(?:\?|$)/.exec(path)||[])[1];try{var response=await fetch(path,options);var body=await response.json();if(operation&&!mutationResult(operation,body,response.status,JSON.parse(options.body),path))throw new Error('Inconclusive mutation result');if(!response.ok){var failure=new Error(body.error||('HTTP '+response.status));failure.body=body;throw failure}return body}catch(error){if(operation&&!error.body){outcomeUnknown=true;sessionReady=false;snapshotCurrent=false;actionsReady=false;error.message='Request outcome unknown; explicitly replace the match to recover'}throw error}}
    function syncControls(){document.querySelectorAll('button').forEach(function(button){if(button.classList.contains('action'))button.disabled=busy||!sessionReady||!snapshotCurrent||!actionsReady});byId('save').disabled=busy||!sessionReady||!snapshotCurrent;byId('replay').disabled=busy||!sessionReady||!snapshotCurrent;byId('stale').disabled=busy||!sessionReady||!snapshotCurrent||!actionsReady||!lastCommand;byId('resume').disabled=busy||!savedPosition;byId('reset-form').querySelector('button').disabled=busy;syncSeatButtons()}
    async function guarded(operation,requiresReady){if(busy||(requiresReady&&(!sessionReady||!snapshotCurrent)))return;busy=true;syncControls();try{await operation()}catch(error){showError(error)}finally{busy=false;syncControls()}}
    function applyResponse(data,operation){if(['reset','resume'].includes(operation)&&data.replacement&&data.replacement.status==='confirmed'){outcomeUnknown=false;snapshot=undefined;lastCommand=undefined;byId('notice').className='ok';byId('notice').textContent='Match replacement confirmed';byId('receipt').textContent=JSON.stringify(data.replacement.receipt,null,2);byId('opponent-summary').textContent='';clearReplaySteps();document.querySelectorAll('[data-cell]').forEach(function(cell){cell.innerHTML=''});['north','south'].forEach(function(owner){byId(owner+'-hand').innerHTML='';byId(owner+'-stats').textContent=''});['active','phase','version','hash'].forEach(function(id){byId(id).textContent='—'})}if(operation==='action'&&data.accepted===true){byId('notice').className='ok';byId('notice').textContent='Action accepted: '+data.playerAction;byId('receipt').textContent=JSON.stringify(data.receipt,null,2);byId('opponent-summary').textContent=data.opponentActions&&data.opponentActions.length?'South actions: '+data.opponentActions.map(function(item){return item.kind.replaceAll('-',' ')+(item.events.length?' — '+item.events.join(' → '):'')}).join('; '):''}else if(operation==='action'&&data.accepted===false){byId('notice').className='error';byId('notice').textContent='Rejected: '+data.reason.message}if(data.recovery&&data.recovery.sessionStatus==='unknown')outcomeUnknown=true;sessionReady=!outcomeUnknown&&(!data.recovery||data.recovery.sessionStatus==='ready');snapshotCurrent=!outcomeUnknown&&!!data.view&&(!data.recovery||data.recovery.snapshotStatus==='current');if(data.recovery){byId('notice').textContent+=(byId('notice').textContent?' · ':'')+data.recovery.message;if(!snapshotCurrent)byId('notice').textContent+=' · Displayed board is historical / stale'}actionsReady=snapshotCurrent&&(!data.recovery||data.recovery.actionsAvailable);if(snapshotCurrent){render(data)}else{byId('actions').innerHTML='<p class="empty">Current actions unavailable. Displayed match is historical / stale.</p>'}syncControls()}
    function clearActionResult(){byId('notice').textContent='';byId('opponent-summary').textContent='';byId('receipt').textContent=''}
    function escapeHtml(value){return String(value).replace(/[&<>"']/g,function(character){return {'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[character]})}
    function cardName(cardId){return snapshot&&snapshot.cardNames&&snapshot.cardNames[cardId]||cardId}
    function cardFactText(cardId){var fact=snapshot&&snapshot.cardFacts&&snapshot.cardFacts[cardId];if(!fact)return '';var parts=[fact.cardType[0].toUpperCase()+fact.cardType.slice(1)];if(fact.elements)parts.push(fact.elements.map(function(value){return value[0].toUpperCase()+value.slice(1)}).join('/'));if(Number.isFinite(fact.manaCost))parts.push(fact.manaCost+' mana');if(fact.thresholds){var short={air:'A',earth:'E',fire:'F',water:'W'},threshold=Object.keys(short).filter(function(key){return fact.thresholds[key]}).map(function(key){return short[key]+fact.thresholds[key]}).join('');if(threshold)parts.push(threshold)}if(fact.cardType==='minion')parts.push(fact.attack+'/'+fact.defense);if(fact.cardType==='avatar')parts.push(fact.attack+' atk · '+fact.life+' life');if(fact.keywords)parts.push(fact.keywords.join(', '));return parts.join(' · ')}
    function displayText(value){var result=String(value);Object.entries(snapshot&&snapshot.cardNames||{}).forEach(function(entry){result=result.replaceAll(entry[0],entry[1])});return result}
    function cardChip(card){var name=escapeHtml(cardName(card.cardId)),facts=escapeHtml(cardFactText(card.cardId));return '<span class="card">'+name+(facts?'<small>'+facts+'</small>':'')+'</span>'}
    function renderPlayer(view,owner){var player=view.players[owner],hand=player.hand,affinity=player.affinity;byId(owner+'-stats').textContent='Life '+player.avatar.life+' · Atlas '+player.atlasCount+' · Spellbook '+player.spellbookCount+' · Cemetery '+player.cemetery.length+' · Mana '+player.mana+' · Affinity E'+affinity.earth+' F'+affinity.fire+' W'+affinity.water+' A'+affinity.air+(player.domainEstablished?' · Domain established':' · Domain pending');var cards=[];['atlas','spellbook'].forEach(function(zone){var value=hand[zone];if(Array.isArray(value)){value.forEach(function(card){cards.push(cardChip(card))})}else{cards.push('<span class="card">'+value+' hidden '+zone+'</span>')}});byId(owner+'-hand').innerHTML=cards.join('')}
    function addRealmPiece(cell,html){var target=document.querySelector('[data-cell="'+cell+'"]');if(target)target.innerHTML+=html}
    function controlText(piece){var controller=piece.controller===null?'neutral':piece.controller;return controller+(piece.owner&&piece.owner!==piece.controller?' · owned '+piece.owner:'')+' · '+(piece.region||'surface')}
    function bearerText(view,bearer){var card=bearer.kind==='avatar'?view.players[bearer.seat].avatar:(view.realm.units||[]).find(function(unit){return unit.instanceId===bearer.instanceId});return card?cardName(card.cardId)+' ('+bearer.seat+')':bearer.kind+' ('+bearer.seat+')'}
    function immobileText(area){return (area.minionsAtSitesOnly?'site minions immobile':'units immobile')+(area.suppressesAirborne?' · grounds airborne':'')+(area.expiresAtSeat?' · until '+area.expiresAtSeat+' turn':'')}
    function renderRealm(view){
      document.querySelectorAll('[data-cell]').forEach(function(cell){cell.innerHTML=''})
      Object.entries(view.realm.sites||{}).forEach(function(entry){var site=entry[1],facts=escapeHtml(cardFactText(site.cardId));addRealmPiece(entry[0],'<span class="piece site">'+escapeHtml(cardName(site.cardId))+(facts?'<small>'+facts+'</small>':'')+'<small>'+escapeHtml(controlText(site))+'</small></span>')})
      ;(view.realm.units||[]).forEach(function(unit){var facts=escapeHtml(cardFactText(unit.cardId)),states=[unit.damage&&unit.damage+' dmg',unit.tapped&&'tapped',unit.summoningSickness&&'new',unit.disabled&&'disabled',unit.immobile&&'immobile',unit.warded&&'warded',unit.stealthed&&'stealthed',unit.airborne&&'airborne'].filter(Boolean);addRealmPiece(unit.location,'<span class="piece unit">'+escapeHtml(cardName(unit.cardId))+(facts?'<small>'+facts+'</small>':'')+'<small>'+escapeHtml(controlText(unit))+'</small><small>Now '+unit.attack+'/'+unit.defense+(states.length?' · '+escapeHtml(states.join(' · ')):'')+'</small></span>');(unit.occupiedCells||[]).filter(function(cell){return cell!==unit.location}).forEach(function(cell){addRealmPiece(cell,'<span class="piece marker" aria-hidden="true">'+escapeHtml(cardName(unit.cardId))+' footprint</span>')})})
      ;(view.realm.artifacts||[]).forEach(function(artifact){var facts=escapeHtml(cardFactText(artifact.cardId)),state=artifact.bearer?'carried by '+bearerText(view,artifact.bearer):'loose';addRealmPiece(artifact.location,'<span class="piece artifact">'+escapeHtml(cardName(artifact.cardId))+(facts?'<small>'+facts+'</small>':'')+'<small>'+escapeHtml(controlText(artifact))+' · '+escapeHtml(state)+'</small></span>')})
      var immobileBySource=new Map((view.realm.immobileAreas||[]).map(function(area){return[area.sourceInstanceId,area]})),auraSources=new Set((view.realm.auras||[]).map(function(aura){return aura.instanceId}));(view.realm.auras||[]).forEach(function(aura){var area=immobileBySource.get(aura.instanceId),effect=area?' · '+immobileText(area):'',facts=escapeHtml(cardFactText(aura.cardId)),footprint=aura.cells.join(' · ');addRealmPiece(aura.cells[0],'<span class="piece aura">'+escapeHtml(cardName(aura.cardId))+(facts?'<small>'+facts+'</small>':'')+'<small>'+escapeHtml(aura.controller+' · '+footprint+' · turn counter '+aura.turnCounters+effect)+'</small></span>');aura.cells.slice(1).forEach(function(cell){addRealmPiece(cell,'<span class="piece marker" aria-hidden="true">'+escapeHtml('Aura area'+effect)+'</span>')})})
      ;(view.realm.immobileAreas||[]).filter(function(area){return !auraSources.has(area.sourceInstanceId)}).forEach(function(area){var label=immobileText(area);area.cells.forEach(function(cell){addRealmPiece(cell,'<span class="piece marker effect">'+escapeHtml(label)+'</span>')})})
      ;['north','south'].forEach(function(owner){var avatar=view.players[owner].avatar;addRealmPiece(avatar.location,'<span class="piece avatar '+owner+'">'+escapeHtml(cardName(avatar.cardId))+'<small>'+owner+' · '+escapeHtml(avatar.region)+' · now '+avatar.attack+' atk · '+avatar.life+' life'+(avatar.tapped?' · tapped':'')+'</small></span>')})
    }
    function actionButton(candidate){var button=document.createElement('button');button.type='button';button.className='action';button.textContent=displayText(candidate.label);button.addEventListener('click',function(){submit(candidate.actionId)});return button}
    function visibleActionButtons(){return Array.from(byId('actions').querySelectorAll('button.action')).filter(function(button){return button.offsetParent!==null})}
    function markActionShortcuts(){visibleActionButtons().forEach(function(button,index){if(index<9)button.setAttribute('aria-keyshortcuts',String(index+1));else button.removeAttribute('aria-keyshortcuts')})}
    function focusAction(button){if(!button)return;visibleActionButtons().forEach(function(item){item.classList.remove('is-focused')});button.classList.add('is-focused');try{button.focus({focusVisible:true})}catch(_error){button.focus()}}
    function focusFirstAction(){focusAction(visibleActionButtons()[0])}
    function actionGroup(kind){if(kind==='draw'||kind==='draw-site'||kind==='draw-spell')return'Draw';if(kind==='play-site')return'Play a site';if(kind==='summon-minion'||kind==='cast-magic'||kind==='cast-artifact'||kind==='cast-aura')return'Play a card';if(kind.startsWith('activate-')||kind==='pick-up-artifacts'||kind==='drop-artifacts')return'Use an ability';if(kind==='move-and-attack')return'Move / attack';if(kind==='declare-attack'||kind==='decline-attack'||kind==='defend'||kind==='close-defend'||kind==='intercept'||kind==='close-intercept'||kind==='allocate-strike')return'Combat';if(kind==='end-turn')return'End turn';return'Resolve effect'}
    function appendActionGroup(dock,label,items){if(items.length<=3){items.forEach(function(item){dock.appendChild(actionButton(item))});return}var details=document.createElement('details'),summary=document.createElement('summary'),list=document.createElement('div');summary.textContent=label+' · '+items.length+' choices';list.className='action-list';items.forEach(function(item){list.appendChild(actionButton(item))});details.append(summary,list);dock.appendChild(details)}
    function renderActions(actions,view){var dock=byId('actions');dock.innerHTML='';if(view.terminal.status==='finished'){var terminal=view.terminal,outcome=terminal.result==='draw'?'Draw':'Winner: '+escapeHtml(terminal.winner)+' · Loser: '+escapeHtml(terminal.loser);dock.innerHTML='<div class="ok" role="status" aria-live="polite"><strong>Game over</strong><p>'+outcome+' · Reason: '+escapeHtml(terminal.reason.replaceAll('_',' '))+'</p></div>';return}if(!actions.length){dock.innerHTML='<p class="empty">No legal actions for this observer.</p>';return}if(view.phase==='mulligan'){var keep=actions.filter(function(a){return a.descriptor.kind==='mulligan'&&!a.descriptor.atlasOrder.length&&!a.descriptor.spellbookOrder.length});keep.forEach(function(a){dock.appendChild(actionButton(a))});var rest=actions.filter(function(a){return keep.indexOf(a)<0});appendActionGroup(dock,'Mulligan alternatives',rest);markActionShortcuts();return}var groups=new Map(),order=['Draw','Play a site','Play a card','Use an ability','Move / attack','Combat','Resolve effect','End turn'];actions.forEach(function(action){var group=actionGroup(action.descriptor.kind);if(!groups.has(group))groups.set(group,[]);groups.get(group).push(action)});Array.from(groups).sort(function(left,right){return order.indexOf(left[0])-order.indexOf(right[0])}).forEach(function(group){appendActionGroup(dock,group[0],group[1])});markActionShortcuts()}
    function render(data){snapshot=data;var view=data.view,picker=byId('preset');if(picker.options.length!==data.presets.length){picker.innerHTML=data.presets.map(function(preset){return '<option value="'+escapeHtml(preset.id)+'">'+escapeHtml(preset.label)+'</option>'}).join('')}picker.value=data.presetId;byId('opponent').value=data.opponent;byId('seed').value=String(data.seed);byId('mode').textContent=(data.mode==='private-local'?'Private-local actual cards · unranked':'Synthetic fallback · unranked')+(data.opponent==='south'?' · vs computer':' · hot seat');byId('observer').textContent=seat;byId('active').textContent=view.activeSeat+(view.decisionSeat===view.activeSeat?'':' · '+view.decisionSeat+' deciding');byId('phase').textContent='Turn '+view.turnNumber+' · '+view.phase;byId('version').textContent=view.stateVersion;byId('hash').textContent=data.stateHash;renderPlayer(view,'north');renderPlayer(view,'south');renderRealm(view);renderActions(data.actions,view);syncSeatButtons()}
    function syncSeatButtons(){document.querySelectorAll('[data-seat]').forEach(function(button){button.disabled=busy||Boolean(snapshot&&snapshot.opponent==='south'&&button.dataset.seat==='south');button.setAttribute('aria-pressed',String(button.dataset.seat===seat))})}
    async function refresh(){applyResponse(await request('/api/view?seat='+seat))}
    async function submit(actionId,command){if(!actionsReady)return;return guarded(async function(){var next=command||{actionId:actionId,seat:seat,stateVersion:snapshot.view.stateVersion};if(!command)lastCommand=next;var result=await request('/api/action',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(next)});applyResponse(result,'action');if(result.accepted&&snapshotCurrent&&sessionReady&&result.opponent==='manual'&&result.view.decisionSeat!==seat){seat=result.view.decisionSeat;syncSeatButtons();await refresh()}},true)}
    function showError(error){if(error.body)applyResponse(error.body);else{snapshotCurrent=false;byId('actions').innerHTML='<p class="empty">Latest snapshot unavailable. Displayed board is historical / stale.</p>'}byId('notice').className='error';byId('notice').textContent+=(byId('notice').textContent?' · ':'')+error.message;syncControls()}
    function replayStepText(step){var events=step.eventTypes.length?step.eventTypes.join(' → '):'no events';return 'Step '+(replayIndex+1)+' of '+replaySteps.length+' · '+step.seat+' · '+events+' · chained hashes '+(replayChained?'match':'do not match')}
    function renderReplayStep(){byId('replay-prev').disabled=!replaySteps.length||replayIndex<=0;byId('replay-next').disabled=!replaySteps.length||replayIndex>=replaySteps.length-1;var step=replaySteps[replayIndex];byId('replay-step').className=step?'':'empty';byId('replay-step').textContent=step?replayStepText(step):'Verify replay to inspect committed steps.';byId('replay-detail').className=step?'receipt':'receipt hidden';byId('replay-detail').textContent=step?JSON.stringify({actionId:step.actionId,index:step.index,nextStateVersion:step.nextStateVersion,postStateHash:step.postStateHash,preStateHash:step.preStateHash,seat:step.seat,stateVersion:step.stateVersion},null,2):''}
    function clearReplaySteps(){replaySteps=[];replayIndex=0;replayChained=false;renderReplayStep()}
    function shiftReplay(delta){if(!replaySteps.length)return;replayIndex=Math.max(0,Math.min(replaySteps.length-1,replayIndex+delta));renderReplayStep()}
    document.querySelectorAll('[data-seat]').forEach(function(button){button.addEventListener('click',function(){if(button.disabled)return;guarded(async function(){seat=button.dataset.seat;syncSeatButtons();await refresh()})})});
    byId('preset').addEventListener('change',function(){if(busy)return;var selected=snapshot&&snapshot.presets.find(function(preset){return preset.id===byId('preset').value});if(selected)byId('seed').value=String(selected.seed)});
    byId('reset-form').addEventListener('submit',function(event){event.preventDefault();guarded(async function(){seat='north';var data=await request('/api/reset',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({opponent:byId('opponent').value,presetId:byId('preset').value,seed:Number(byId('seed').value)})});applyResponse(data,'reset');focusFirstAction()})});
    byId('stale').addEventListener('click',function(){if(lastCommand)submit(lastCommand.actionId,lastCommand)});
    byId('save').addEventListener('click',function(){guarded(async function(){var data=await request('/api/checkpoint',{method:'POST'});savedPosition=data;localStorage.setItem(saveKey,JSON.stringify(data));byId('notice').textContent+=' · Position saved in this browser at turn '+data.turnNumber},true)});
    byId('resume').addEventListener('click',function(){if(!savedPosition)return;guarded(async function(){var data=await request('/api/resume?seat='+encodeURIComponent(seat)+'&opponent='+encodeURIComponent(savedPosition.opponent),{method:'POST',headers:{'content-type':'application/json'},body:savedPosition.checkpoint});applyResponse(data,'resume')})});
    byId('replay-prev').addEventListener('click',function(){shiftReplay(-1)});
    byId('replay-next').addEventListener('click',function(){shiftReplay(1)});
    document.addEventListener('keydown',function(event){var tag=(event.target&&event.target.tagName||'').toLowerCase();if(tag==='input'||tag==='select'||tag==='textarea')return;if(event.key==='ArrowLeft'||event.key==='ArrowRight'){event.preventDefault();shiftReplay(event.key==='ArrowLeft'?-1:1);return}if(event.key==='ArrowDown'||event.key==='ArrowUp'){var buttons=visibleActionButtons();if(!buttons.length)return;event.preventDefault();var index=buttons.indexOf(document.activeElement);var next=event.key==='ArrowDown'?(index<0?0:index+1):(index<0?buttons.length-1:index-1);focusAction(buttons[Math.max(0,Math.min(buttons.length-1,next))]);return}if(/^[1-9]$/.test(event.key)){var chosen=visibleActionButtons()[Number(event.key)-1];if(!chosen)return;event.preventDefault();chosen.click()}});
    byId('replay').addEventListener('click',function(){guarded(async function(){var data=await request('/api/replay',{method:'POST'});var steps=await request('/api/replay/steps');replaySteps=Array.isArray(steps.steps)?steps.steps:[];replayIndex=replaySteps.length?replaySteps.length-1:0;replayChained=!!steps.chained;byId('notice').textContent+=' · '+(data.verified?'Replay byte-identical':'Replay mismatch');renderReplayStep()},true)});
    guarded(async function(){await refresh();focusFirstAction()});
  </script>
</body>
</html>`;

type RecoveryStage = 'player-presentation' | 'opponent-budget' | 'opponent-policy' | 'opponent-step' | 'snapshot' | 'replacement';
type SnapshotStatus = 'current' | 'unavailable' | 'untrusted';

function validHash(value: unknown): value is Sha256Hash {
  return typeof value === 'string' && /^sha256:[0-9a-f]{64}$/.test(value);
}

type JsonRecord = Record<string, unknown>;

function displayCardFacts(card: GameCardDefinition): JsonRecord {
  const facts: JsonRecord = { cardType: card.cardType };
  if ('manaCost' in card) {
    facts.manaCost = card.manaCost;
    facts.thresholds = card.thresholds;
  }
  if ('attack' in card) {
    facts.attack = card.attack;
    facts.defense = card.defense;
  }
  if (card.cardType === 'avatar') facts.life = card.life;
  if (card.cardType === 'site') {
    facts.elements = card.elements;
    const keywords = [
      card.airborneMinionsAtopMoveFreelyAway && 'Airborne minions move freely away',
      card.blocksGroundMinionEntryWhileMinionAtop && 'Occupied: blocks ground minion entry',
    ].filter((keyword): keyword is string => Boolean(keyword));
    if (keywords.length > 0) facts.keywords = keywords;
  }
  if (card.cardType === 'minion') {
    const keywords = [
      card.airborne && 'Airborne',
      card.burrowing && 'Burrowing',
      card.charge && 'Charge',
      card.immobile && 'Immobile',
      card.lethal && 'Lethal',
      card.ranged && 'Ranged',
      card.spellcaster && 'Spellcaster',
      card.stealth && 'Stealth',
      card.submerge && 'Submerge',
      card.voidwalk && 'Voidwalk',
      card.waterbound && 'Waterbound',
      card.ward && 'Ward',
    ].filter((keyword): keyword is string => Boolean(keyword));
    if (keywords.length > 0) facts.keywords = keywords;
  }
  return facts;
}

function visibleCardIds(observation: GameObservation): ReadonlySet<string> {
  const players = Object.values(observation.players);
  return new Set([
    ...players.flatMap((player) => [
      player.avatar.cardId,
      ...player.cemetery.map(({ cardId }) => cardId),
      ...(Array.isArray(player.hand.atlas) ? player.hand.atlas.map(({ cardId }) => cardId) : []),
      ...(Array.isArray(player.hand.spellbook) ? player.hand.spellbook.map(({ cardId }) => cardId) : []),
    ]),
    ...Object.values(observation.realm.sites).map(({ cardId }) => cardId),
    ...observation.realm.units.map(({ cardId }) => cardId),
    ...(observation.realm.artifacts ?? []).map(({ cardId }) => cardId),
    ...(observation.realm.auras ?? []).map(({ cardId }) => cardId),
  ]);
}

function isSeat(value: unknown): value is GameSeat {
  return value === 'north' || value === 'south';
}

function reseedManifest(manifest: GameManifest, seed: number): GameManifest {
  return createGameManifest({
    authority: manifest.authority,
    cards: manifest.cards,
    decks: manifest.decks,
    firstSeat: manifest.firstSeat,
    seed,
  });
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function parseExportedSession(exported: JsonValue, manifest: GameManifest): GameSession {
  if (!isRecord(exported)
    || !Array.isArray(exported.attempts)
    || !Array.isArray(exported.initialRandomDraws)
    || !Array.isArray(exported.transcript)
    || !isRecord(exported.state)) {
    throw new Error('Rust exportSession result did not match GameSession');
  }
  return {
    attempts: exported.attempts as GameSession['attempts'],
    initialRandomDraws: exported.initialRandomDraws as GameSession['initialRandomDraws'],
    manifest,
    state: exported.state as GameSession['state'],
    transcript: exported.transcript as GameSession['transcript'],
  };
}

function asGameLegalActions(actions: readonly RustLegalAction[]): readonly GameLegalAction[] {
  return actions as readonly GameLegalAction[];
}

function displayActionLabel(
  action: GameLegalAction,
  observation: GameObservation,
  cardNames: Readonly<Record<string, string>>,
  cards: GameManifest['cards'],
): string {
  const ownHand = observation.players[observation.viewer].hand;
  const handCards = [
    ...(Array.isArray(ownHand.atlas) ? ownHand.atlas : []),
    ...(Array.isArray(ownHand.spellbook) ? ownHand.spellbook : []),
  ];
  const handNames = new Map(handCards.map(({ cardId, instanceId }) =>
    [instanceId, cardNames[cardId] ?? cardId]));
  if (action.descriptor.kind === 'mulligan') {
    if (action.descriptor.atlasOrder.length === 0
      && action.descriptor.spellbookOrder.length === 0) return action.label;
    const selected = (zone: 'atlas' | 'spellbook', instanceIds: readonly string[]): string =>
      instanceIds.length === 0
        ? ''
        : `${zone}: ${instanceIds.map((instanceId) =>
          handNames.get(instanceId) ?? instanceId.slice(0, 15) + '…').join(' → ')}`;
    return ['Mulligan', selected('atlas', action.descriptor.atlasOrder),
      selected('spellbook', action.descriptor.spellbookOrder)].filter(Boolean).join(' · ');
  }

  const references = new Map<string, string>();
  for (const unit of observation.realm.units) {
    references.set(
      unit.instanceId,
      `${cardNames[unit.cardId] ?? unit.cardId} · ${unit.controller} · ${unit.location} ${unit.region}`,
    );
  }
  for (const seat of ['north', 'south'] as const) {
    const avatar = observation.players[seat].avatar;
    references.set(
      avatar.instanceId,
      `${cardNames[avatar.cardId] ?? avatar.cardId} · ${seat} · ${avatar.location} ${avatar.region}`,
    );
  }
  for (const [cell, site] of Object.entries(observation.realm.sites)) {
    references.set(
      site.instanceId,
      `${cardNames[site.cardId] ?? site.cardId} · ${site.controller ?? 'neutral'} · ${cell} surface`,
    );
  }
  for (const seat of ['north', 'south'] as const) {
    for (const card of observation.players[seat].cemetery) {
      references.set(card.instanceId, `${cardNames[card.cardId] ?? card.cardId} · ${seat} cemetery`);
    }
  }
  for (const artifact of observation.realm.artifacts ?? []) {
    const bearer = artifact.bearer
      ? ` · carried by ${references.get(artifact.bearer.instanceId)
        ?? `${artifact.bearer.kind} · ${artifact.bearer.seat}`}`
      : ' · loose';
    references.set(
      artifact.instanceId,
      `${cardNames[artifact.cardId] ?? artifact.cardId} · ${artifact.controller ?? 'neutral'} · ${artifact.location} ${artifact.region}${bearer}`,
    );
  }
  for (const aura of observation.realm.auras ?? []) {
    references.set(
      aura.instanceId,
      `${cardNames[aura.cardId] ?? aura.cardId} · ${aura.controller} · ${aura.cells.join('/')} surface · turn counter ${aura.turnCounters}`,
    );
  }

  let label = action.label;
  for (const [instanceId, display] of references) {
    label = label
      .replaceAll(instanceId, display)
      .replaceAll(instanceId.slice(0, 15) + '…', display);
  }
  for (const [cardId, name] of Object.entries(cardNames)
    .sort(([left], [right]) => right.length - left.length)) {
    label = label.replaceAll(cardId, name);
  }
  const descriptor = action.descriptor;
  if (descriptor.kind === 'play-site') {
    const definition = cards[descriptor.cardId];
    if (definition?.cardType === 'site'
      && definition.genesisPayOneManaToSummonToken !== undefined) {
      const siteName = cardNames[descriptor.cardId] ?? 'this site';
      const tokenName = cardNames[definition.genesisPayOneManaToSummonToken] ?? 'a token';
      const rubble = descriptor.createRubbleAt
        ? ` and create Rubble at ${descriptor.createRubbleAt}`
        : '';
      return descriptor.genesisTokenChoice === 'pay-one-mana'
        ? `Play ${siteName} at ${descriptor.cell} — spend 1 mana to summon ${tokenName} there${rubble}`
        : `Play ${siteName} at ${descriptor.cell} — keep 1 mana and summon no ${tokenName}${rubble}`;
    }
    if (definition?.cardType === 'site' && definition.genesisMayBottomNextSpell === true) {
      return `${label} — then inspect the next spell`;
    }
  }
  if (descriptor.kind === 'cast-magic') {
    const definition = cards[descriptor.cardId];
    if (definition?.cardType === 'magic'
      && definition.teleportNearbyAllyThenDrawCard === true
      && descriptor.drawZone) {
      return `${label} — then draw from ${descriptor.drawZone === 'atlas' ? 'Atlas' : 'Spellbook'}`;
    }
    if (definition?.cardType === 'magic'
      && definition.grantChargeToAllyThisTurn === true) {
      return `${label} — the ally can move and attack this turn`;
    }
    if (definition?.cardType === 'magic' && definition.damageTargetUnit !== undefined) {
      return `${label} — attempt to deal ${definition.damageTargetUnit} damage`;
    }
  }
  return label;
}

async function readText(
  request: IncomingMessage,
  maximumBytes = MAX_BODY_BYTES,
): Promise<string> {
  const chunks: Buffer[] = [];
  let size = 0;
  for await (const chunk of request) {
    const buffer = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk as Uint8Array);
    size += buffer.length;
    if (size > maximumBytes) throw new Error(`request body exceeds ${maximumBytes} bytes`);
    chunks.push(buffer);
  }
  return Buffer.concat(chunks).toString('utf8');
}

async function readJson(request: IncomingMessage): Promise<JsonRecord> {
  const parsed: unknown = JSON.parse(await readText(request) || '{}');
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error('request body must be a JSON object');
  }
  return parsed as JsonRecord;
}

function sendJson(response: ServerResponse, status: number, value: unknown): void {
  const body = JSON.stringify(value);
  response.writeHead(status, {
    'cache-control': 'no-store',
    'content-length': Buffer.byteLength(body),
    'content-type': 'application/json; charset=utf-8',
    'x-content-type-options': 'nosniff',
  });
  response.end(body);
}

function sendPage(response: ServerResponse, presets: readonly GamePreset[]): void {
  // Configuration choices remain available before the first trustworthy match view.
  const escape = (text: string): string => text.replace(/[&<>"']/g, (character) =>
    ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[character]!);
  const page = PAGE.replace('<select id="preset" aria-label="Starter matchup"></select>',
    `<select id="preset" aria-label="Starter matchup">${presets.map(({ id, label }) =>
      `<option value="${id}">${escape(label)}</option>`).join('')}</select>`);
  response.writeHead(200, {
    'cache-control': 'no-store',
    'content-length': Buffer.byteLength(page),
    'content-security-policy': "default-src 'self'; connect-src 'self'; img-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; base-uri 'none'; frame-ancestors 'none'",
    'content-type': 'text/html; charset=utf-8',
    'x-content-type-options': 'nosniff',
  });
  response.end(page);
}

export function createGamePrototypeServer(
  initialSeed?: number,
  suppliedPresets?: readonly GamePreset[],
  initialOpponent: GameOpponent = 'manual',
): Server {
  const presets = suppliedPresets?.length
    ? [...suppliedPresets]
    : [{ id: 'synthetic', label: 'Synthetic fallback', manifest: createSyntheticDemoManifest(1) }];
  if (presets.length > 16
    || new Set(presets.map(({ id }) => id)).size !== presets.length
    || presets.some(({ id, label }) => !/^[a-z][a-z0-9-]{0,31}$/.test(id) || !label || label.length > 80)) {
    throw new RangeError('presets must contain 1-16 unique labeled IDs');
  }
  let selectedPreset = presets[0]!;
  let opponent = initialOpponent;
  let manifest = reseedManifest(selectedPreset.manifest, initialSeed ?? selectedPreset.manifest.seed);
  const clientPromise = RustSessionClient.start();
  let binding: 'uninitialized' | 'bound' | 'unknown' = 'uninitialized';
  let integrity: 'coherent' | 'unsupported' | 'unknown' = 'coherent';
  let blocked = false;
  let pair: { stateHash: Sha256Hash; stateVersion: number } | undefined;
  let operationQueue: Promise<unknown> = Promise.resolve();

  function queue<T>(operation: () => Promise<T>): Promise<T> {
    const pending = operationQueue.then(operation);
    operationQueue = pending.catch(() => undefined);
    return pending;
  }

  function sessionStatus(): 'ready' | 'blocked' | 'unknown' {
    return binding !== 'bound' || integrity === 'unknown'
      ? 'unknown' : blocked || integrity === 'unsupported' ? 'blocked' : 'ready';
  }

  function recovery(stage: RecoveryStage, message: string, snapshotStatus: SnapshotStatus,
    actionsAvailable = false): JsonRecord {
    return { stage, message, snapshotStatus, actionsAvailable, sessionStatus: sessionStatus(),
      metadataStatus: binding === 'bound' ? 'known' : 'unknown' };
  }

  function stepFailure(error: unknown): string {
    const effect = error instanceof RustSessionRpcError && error.method === 'step'
      ? error.sessionEffect : 'unknown';
    if (effect === 'capacity') {
      blocked = true;
      return 'session request limit of 1000 reached';
    }
    if (effect === 'unsupported') {
      integrity = 'unsupported';
      return 'An unsupported mechanic stopped this session';
    }
    integrity = 'unknown';
    return 'The action outcome is unknown; explicitly replace the match to recover';
  }

  function confirmReceipt(value: unknown, request: { actionId: string; seat: GameSeat; stateVersion: number }): GameSession['transcript'][number] {
    const integer = (value: unknown): value is number => Number.isSafeInteger(value) && (value as number) >= 0;
    if (!pair || !isRecord(value) || !validHash(value.actionId) || value.actionId !== request.actionId || value.seat !== request.seat
      || value.stateVersion !== request.stateVersion || value.stateVersion !== pair.stateVersion
      || value.preStateHash !== pair.stateHash || !integer(value.nextStateVersion)
      || value.nextStateVersion !== pair.stateVersion + 1 || !validHash(value.postStateHash)
      || !validHash(value.receiptId) || !integer(value.receiptSequence) || value.receiptSequence < 1
      || !Array.isArray(value.events) || !Array.isArray(value.randomDraws)) {
      throw new Error('Incomplete native receipt');
    }
    for (const event of value.events) {
      if (!isRecord(event) || !isRecord(event.cause) || event.cause.actionId !== value.actionId
        || event.cause.receiptSequence !== value.receiptSequence || !integer(event.eventSequence)
        || !validHash(event.eventId) || typeof event.type !== 'string' || event.payload === undefined) {
        throw new Error('Incomplete native event');
      }
      const { eventId, ...body } = event;
      if (identityHash(body as JsonValue) !== eventId) throw new Error('Invalid native event identity');
    }
    for (const draw of value.randomDraws) {
      if (!isRecord(draw) || !integer(draw.drawSequence) || typeof draw.purpose !== 'string'
        || draw.domain === undefined || draw.result === undefined
        || !validHash(draw.prePrngStateHash) || !validHash(draw.postPrngStateHash)) {
        throw new Error('Incomplete native random draw');
      }
    }
    const { receiptId, ...body } = value;
    if (identityHash(body as JsonValue) !== receiptId) throw new Error('Invalid native receipt identity');
    pair = { stateHash: value.postStateHash, stateVersion: value.nextStateVersion };
    return value as unknown as GameSession['transcript'][number];
  }

  async function observation(client: RustSessionClient, seat: GameSeat): Promise<GameObservation> {
    if (binding !== 'bound' || integrity === 'unknown' || !pair) throw new Error('Untrusted session');
    const viewed = await client.publicView(seat);
    if (viewed.stateHash !== pair.stateHash
      || (isRecord(viewed.view) && Number.isSafeInteger(viewed.view.stateVersion)
        && (viewed.view.stateVersion as number) >= 0
        && viewed.view.stateVersion !== pair.stateVersion)) {
      integrity = 'unknown';
      throw new Error('Incoherent session view');
    }
    if (!isRecord(viewed.view) || !Number.isSafeInteger(viewed.view.stateVersion)
      || (viewed.view.stateVersion as number) < 0
      || !isSeat(viewed.view.decisionSeat) || viewed.view.viewer !== seat
      || !isRecord(viewed.view.terminal)
      || (viewed.view.terminal.status !== 'active' && viewed.view.terminal.status !== 'finished')) {
      throw new Error('Malformed session view');
    }
    return viewed.view as unknown as GameObservation;
  }

  async function snapshot(client: RustSessionClient, seat: GameSeat,
    diagnostic?: { stage: RecoveryStage; message: string }): Promise<JsonRecord> {
    if (binding !== 'bound' || integrity === 'unknown') {
      return { recovery: recovery(diagnostic?.stage ?? 'snapshot', diagnostic?.message
        ?? 'Current state is untrusted; explicitly replace the match to recover', 'untrusted') };
    }
    try {
      const viewed = await observation(client, seat);
      const visible = visibleCardIds(viewed);
      const cardNames = Object.fromEntries(Object.entries(selectedPreset.cardNames ?? {})
        .filter(([cardId]) => visible.has(cardId)));
      const cardFacts = Object.fromEntries(Object.entries(manifest.cards)
        .filter(([cardId]) => visible.has(cardId)).map(([id, card]) => [id, displayCardFacts(card)]));
      let actions: readonly GameLegalAction[] = [];
      let actionsAvailable = integrity === 'coherent';
      if (actionsAvailable) {
        try {
          actions = asGameLegalActions(await client.legalActions(seat));
          actions = actions.map((action) => ({ ...action,
            label: displayActionLabel(action, viewed, selectedPreset.cardNames ?? {}, manifest.cards) }));
        } catch {
          actionsAvailable = false;
          diagnostic ??= { stage: 'snapshot', message: 'Legal actions are unavailable' };
        }
      }
      if (sessionStatus() === 'blocked') diagnostic ??= { stage: 'snapshot',
        message: integrity === 'unsupported' ? 'An unsupported mechanic stopped this session'
          : 'Further submissions are blocked for this session' };
      return { actions, cardFacts, cardNames, mode: manifest.authority.mode, opponent,
        presetId: selectedPreset.id, presets: presets.map(({ id, label, manifest }) => ({ id, label, seed: manifest.seed })),
        seed: manifest.seed, stateHash: pair!.stateHash, view: viewed,
        ...(diagnostic ? { recovery: recovery(diagnostic.stage, diagnostic.message, 'current', actionsAvailable) } : {}) };
    } catch {
      return { recovery: recovery(diagnostic?.stage ?? 'snapshot', diagnostic?.message
        ?? 'The latest snapshot is unavailable', sessionStatus() === 'unknown' ? 'untrusted' : 'unavailable') };
    }
  }

  async function replace(client: RustSessionClient, preset: GamePreset, nextOpponent: GameOpponent,
    candidate: GameManifest, checkpoint?: ReturnType<typeof parseGameCheckpoint>): Promise<{ status: number; data: JsonRecord }> {
    const method = checkpoint ? 'resume' : 'new';
    try {
      const receipt: JsonRecord = checkpoint
        ? await client.resume(checkpoint as unknown as JsonValue)
        : await client.newSession(canonicalJson(candidate as unknown as JsonValue));
      if (!isRecord(receipt) || !validHash(receipt.stateHash) || !Number.isSafeInteger(receipt.stateVersion)
        || (receipt.stateVersion as number) < 0
        || (checkpoint ? receipt.expectedSessionHash !== checkpoint.expectedSessionHash
          : receipt.manifestId !== candidate.manifestId)) throw new Error('Invalid replacement acknowledgement');
      selectedPreset = preset;
      opponent = nextOpponent;
      manifest = candidate;
      pair = { stateHash: receipt.stateHash, stateVersion: receipt.stateVersion as number };
      binding = 'bound';
      integrity = 'coherent';
      blocked = false;
      return { status: 200, data: { replacement: { status: 'confirmed', receipt } } };
    } catch (error) {
      if (error instanceof RustSessionRpcError && error.method === method && error.sessionEffect === 'unchanged') {
        return { status: 400, data: { error: 'Replacement failed; the previous match is unchanged',
          replacement: { status: 'unchanged' }, recovery: recovery('replacement',
            'Replacement failed; the previous match is unchanged', binding === 'bound' && integrity !== 'unknown' ? 'unavailable' : 'untrusted') } };
      }
      binding = 'unknown';
      return { status: 503, data: { error: 'Replacement outcome is unknown', replacement: { status: 'unknown' },
        recovery: recovery('replacement', 'Replacement outcome is unknown; explicitly replace the match to recover', 'untrusted') } };
    }
  }

  async function ensureSession(client: RustSessionClient): Promise<{ status: number; data: JsonRecord } | undefined> {
    if (binding !== 'uninitialized') return undefined;
    const result = await replace(client, selectedPreset, opponent, manifest);
    return result.status === 200 ? undefined : result;
  }

  const server = createServer(async (request, response) => {
    try {
      const url = new URL(request.url ?? '/', `http://${HOST}`);
      if (request.method === 'GET' && url.pathname === '/') return sendPage(response, presets);
      // Bounded parsing is outside the session queue; all defaults and guards are inside it.
      const body = request.method === 'POST' && ['/api/action', '/api/reset'].includes(url.pathname)
        ? await readJson(request) : undefined;
      const checkpoint = request.method === 'POST' && url.pathname === '/api/resume'
        ? parseGameCheckpoint(await readText(request, GAME_CHECKPOINT_MAX_BYTES)) : undefined;
      return await queue(async () => {
        let client: RustSessionClient;
        try { client = await clientPromise; }
        catch {
          binding = 'unknown';
          return sendJson(response, 503, { error: 'Native session process is unavailable',
            recovery: recovery('snapshot', 'Native session process is unavailable; restart is required', 'untrusted') });
        }
        if (request.method === 'POST' && url.pathname === '/api/reset' && body) {
          if (!Number.isInteger(body.seed) || (body.seed as number) < 0 || (body.seed as number) > 0xffff_ffff) {
            return sendJson(response, 400, { error: 'seed must be an unsigned 32-bit integer' });
          }
          const presetId = body.presetId ?? (binding === 'unknown' ? undefined : selectedPreset.id);
          const preset = presets.find(({ id }) => id === presetId);
          const requestedOpponent = body.opponent ?? (binding === 'unknown' ? undefined : opponent);
          if (!preset) return sendJson(response, 400, { error: 'presetId must name an available preset' });
          if (requestedOpponent !== 'manual' && requestedOpponent !== 'south') {
            return sendJson(response, 400, { error: 'opponent must be manual or south' });
          }
          const result = await replace(client, preset, requestedOpponent, reseedManifest(preset.manifest, body.seed as number));
          return sendJson(response, result.status, { ...(result.status === 200 ? await snapshot(client, 'north') : {}), ...result.data });
        }
        if (request.method === 'POST' && url.pathname === '/api/resume' && checkpoint) {
          const seat = url.searchParams.get('seat');
          const requestedOpponent = url.searchParams.get('opponent');
          if (!isSeat(seat) || (requestedOpponent !== 'manual' && requestedOpponent !== 'south')) {
            return sendJson(response, 400, { error: 'seat and opponent are required' });
          }
          if (requestedOpponent === 'south' && seat === 'south') return sendJson(response, 403, { error: 'south is hidden while controlled by the deterministic opponent' });
          const preset = presets.find(({ manifest }) => reseedManifest(manifest, checkpoint.manifest.seed).manifestId === checkpoint.manifest.manifestId);
          if (!preset) return sendJson(response, 400, { error: 'saved position preset is unavailable' });
          const result = await replace(client, preset, requestedOpponent, checkpoint.manifest, checkpoint);
          return sendJson(response, result.status, { ...(result.status === 200 ? await snapshot(client, seat) : {}), ...result.data });
        }
        if (!['/api/view', '/api/action', '/api/checkpoint', '/api/replay', '/api/replay/steps'].includes(url.pathname)) return sendJson(response, 404, { error: 'not found' });
        const initialized = await ensureSession(client);
        if (initialized) return sendJson(response, initialized.status, initialized.data);
        if (binding !== 'bound' || integrity === 'unknown') return sendJson(response, 503, {
          error: 'Current session is untrusted', recovery: recovery('snapshot', 'Explicitly replace the match to recover', 'untrusted') });
        if (request.method === 'GET' && url.pathname === '/api/view') {
          const seat = url.searchParams.get('seat');
          if (!isSeat(seat)) return sendJson(response, 400, { error: 'seat must be north or south' });
          if (opponent === 'south' && seat === 'south') return sendJson(response, 403, { error: 'south is hidden while controlled by the deterministic opponent' });
          const data = await snapshot(client, seat);
          return sendJson(response, data.view ? 200 : 503, data.view ? data : { error: 'Current snapshot unavailable', ...data });
        }
        if (request.method === 'POST' && url.pathname === '/api/action' && body) {
          const seat = body.seat;
          if (!isSeat(seat) || !Number.isSafeInteger(body.stateVersion) || (body.stateVersion as number) < 0
            || typeof body.actionId !== 'string' || !body.actionId || body.actionId.length > 256
            || Object.keys(body).sort().join(',') !== 'actionId,seat,stateVersion') {
            return sendJson(response, 400, { error: 'valid seat, stateVersion, and actionId are required' });
          }
          if (opponent === 'south' && seat === 'south') return sendJson(response, 400, { error: 'south is controlled by the deterministic opponent' });
          if (sessionStatus() !== 'ready') return sendJson(response, 400, { error: 'Further submissions are blocked', ...await snapshot(client, seat) });
          let playerAction = body.actionId;
          try {
            const viewed = await observation(client, seat);
            const action = asGameLegalActions(await client.legalActions(seat)).find(({ actionId }) => actionId === body.actionId);
            if (action) playerAction = displayActionLabel(action, viewed, selectedPreset.cardNames ?? {}, manifest.cards);
          } catch {
            return sendJson(response, 503, { error: 'Action presentation unavailable', ...await snapshot(client, seat, { stage: 'player-presentation', message: 'Action presentation unavailable' }) });
          }
          const actionRequest = { actionId: body.actionId, seat, stateVersion: body.stateVersion as number };
          let result;
          let receipt;
          try {
            result = await client.step(actionRequest);
            if (result.accepted) receipt = confirmReceipt(result.receipt, actionRequest);
          } catch (error) {
            const message = stepFailure(error);
            return sendJson(response, sessionStatus() === 'unknown' ? 503 : 400, { error: message,
              ...await snapshot(client, seat, { stage: 'opponent-step', message }) });
          }
          if (!result.accepted) return sendJson(response, 200, { ...await snapshot(client, seat), accepted: false,
            reason: result.rejection, opponentActionCount: 0, opponentActions: [] });
          const summaries: Array<{ events: readonly string[]; kind: GameLegalAction['descriptor']['kind'] }> = [];
          let diagnostic: { stage: RecoveryStage; message: string } | undefined;
          let presented: JsonRecord;
          try {
            if (opponent === 'south') {
              while (true) {
                let viewed;
                try {
                  viewed = await observation(client, 'north');
                  if (viewed.terminal.status !== 'active' || viewed.decisionSeat !== 'south') break;
                } catch { diagnostic = { stage: 'snapshot', message: 'Latest opponent state is unavailable' }; break; }
                if (summaries.length >= MAX_OPPONENT_ACTIONS) {
                  blocked = true;
                  diagnostic = { stage: 'opponent-budget', message: 'Deterministic opponent exceeded the request action budget' }; break;
                }
                let action: GameLegalAction;
                try { action = asGameLegalActions([await client.selectPolicyAction()])[0]!; }
                catch { diagnostic = { stage: 'opponent-policy', message: 'Deterministic opponent policy is unavailable' }; break; }
                try {
                  const stepped = await client.step(action);
                  if (!stepped.accepted) { diagnostic = { stage: 'opponent-step', message: 'Deterministic opponent action was rejected' }; break; }
                  const confirmed = confirmReceipt(stepped.receipt, action);
                  summaries.push({ events: confirmed.events.map(({ type }) => type), kind: action.descriptor.kind });
                } catch (error) { diagnostic = { stage: 'opponent-step', message: stepFailure(error) }; break; }
              }
            }
            presented = await snapshot(client, seat, diagnostic);
          } catch {
            presented = { recovery: recovery('snapshot', 'Latest opponent presentation is unavailable',
              sessionStatus() === 'unknown' ? 'untrusted' : 'unavailable') };
          }
          return sendJson(response, 200, { ...presented, accepted: true,
            playerAction, receipt, opponentActionCount: summaries.length, opponentActions: summaries });
        }
        if (sessionStatus() !== 'ready') return sendJson(response, 503, { error: 'Session continuation and certification are blocked',
          recovery: recovery('snapshot', 'Session continuation and certification are blocked', 'unavailable') });
        try {
          if (request.method === 'POST' && url.pathname === '/api/checkpoint') {
            const checkpointValue = await client.checkpoint();
            if (!isRecord(checkpointValue) || !validHash(checkpointValue.checkpointId) || !validHash(checkpointValue.expectedSessionHash)) throw new Error('Invalid checkpoint');
            const viewed = await observation(client, 'north');
            return sendJson(response, 200, { checkpoint: canonicalJson(checkpointValue as JsonValue),
              checkpointId: checkpointValue.checkpointId, opponent, stateHash: pair!.stateHash, turnNumber: viewed.turnNumber });
          }
          if (request.method === 'GET' && url.pathname === '/api/replay/steps') return sendJson(response, 200, await client.replaySteps());
          if (request.method === 'POST' && url.pathname === '/api/replay') {
            const session = parseExportedSession(await client.exportSession(), manifest);
            await observation(client, 'north');
            return sendJson(response, 200, { acceptedActionCount: session.transcript.length, finalStateHash: pair!.stateHash, verified: await client.verifyReplay() });
          }
        } catch {
          return sendJson(response, 503, { error: 'Session evidence unavailable', recovery: recovery('snapshot',
            'Session evidence unavailable', sessionStatus() === 'unknown' ? 'untrusted' : 'unavailable') });
        }
        return sendJson(response, 404, { error: 'not found' });
      });
    } catch {
      return sendJson(response, 400, { error: request.url?.startsWith('/api/resume')
        ? 'Invalid checkpoint request' : 'Invalid request' });
    }
  });
  server.on('close', () => { void clientPromise.then((client) => client.close()); });
  return server;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  let presets: readonly GamePreset[] | undefined;
  try {
    const { loadPrivateStarterCatalog } = await import('../commands/run-private-game-check.ts');
    presets = await loadPrivateStarterCatalog();
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
  }
  createGamePrototypeServer(undefined, presets, 'south').listen(DEFAULT_PORT, HOST, () => {
    process.stdout.write(`Sorcery Playable Core: http://${HOST}:${DEFAULT_PORT}\n`);
  });
}
