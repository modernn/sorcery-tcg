import { createServer, type IncomingMessage, type Server, type ServerResponse } from 'node:http';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  createDemoManifest,
  createDemoSession,
  hashDemoState,
  legalDemoActions,
  observeDemo,
  replayDemo,
  stepDemo,
  verifyDemoReplay,
  type DemoSeat,
  type DemoManifest,
  type DemoSession,
} from '../engine/demo-contract.ts';

const HOST = '127.0.0.1';
const DEFAULT_PORT = 4173;
const MAX_BODY_BYTES = 65_536;

const PAGE = String.raw`<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="initial-scale=1,width=device-width">
  <title>Sorcery Simulator — Engine Lab</title>
  <style>
    :root{color-scheme:dark;--sea:#101b22;--slate:#1e3038;--paper:#e8e0cc;--red:#c44c3b;--blue:#2e6f89;--moss:#697a52;--ink:#dce6e4;--muted:#91a5a6;--line:#38505a;--focus:#f4bd63;font-family:Bahnschrift,"Segoe UI",sans-serif;background:var(--sea);color:var(--ink)}
    *{box-sizing:border-box}body{margin:0;min-height:100vh;background:radial-gradient(circle at 50% 12%,#24424d 0,transparent 36%),linear-gradient(155deg,#0a1217,#101b22 58%,#18282f)}
    button,input{font:inherit}button{color:var(--ink);background:#263d47;border:1px solid #52707b;border-radius:.35rem;padding:.62rem .82rem;cursor:pointer}button:hover{background:#31505d}button:disabled{opacity:.45;cursor:not-allowed}button:focus-visible,input:focus-visible{outline:3px solid var(--focus);outline-offset:3px}
    header{display:flex;justify-content:space-between;align-items:flex-start;gap:1rem;padding:1.2rem clamp(1rem,4vw,3rem);border-bottom:1px solid var(--line);background:#0d181ed9;box-shadow:0 1rem 3rem #0006}h1{font-size:clamp(1.35rem,3vw,2.2rem);letter-spacing:.055em;margin:.1rem 0;text-transform:uppercase}.eyebrow{color:#82adbd;font-size:.76rem;letter-spacing:.16em;text-transform:uppercase}.badges{display:flex;gap:.45rem;flex-wrap:wrap;justify-content:flex-end}.badge{border:1px solid #61727a;border-radius:999px;padding:.3rem .55rem;font:700 .7rem "Cascadia Mono",monospace;letter-spacing:.06em;text-transform:uppercase}.badge.warning{border-color:var(--red);color:#f0a498}.badge.pending{border-color:var(--focus);color:#f4cf8a}
    .toolbar{display:flex;flex-wrap:wrap;align-items:end;gap:.7rem;padding:.8rem clamp(1rem,4vw,3rem);background:#14242c;border-bottom:1px solid var(--line)}label{display:grid;gap:.3rem;color:var(--muted);font-size:.78rem;letter-spacing:.08em;text-transform:uppercase}input{width:12rem;background:#091318;border:1px solid #52707b;border-radius:.3rem;color:var(--ink);padding:.58rem}.seat-switch{display:flex;margin-left:auto}.seat-switch button{border-radius:0}.seat-switch button:first-child{border-radius:.35rem 0 0 .35rem}.seat-switch button:last-child{border-radius:0 .35rem .35rem 0}.seat-switch [aria-pressed=true]{background:var(--blue);border-color:#8bc0d4}
    .shell{display:grid;grid-template-columns:minmax(0,1fr) minmax(17rem,24rem);gap:1rem;padding:1rem clamp(1rem,4vw,3rem) 2rem;max-width:1500px;margin:auto}.table{position:relative;min-height:38rem;border:1px solid #45606b;border-radius:.6rem;overflow:hidden;background:linear-gradient(90deg,#ffffff05 1px,transparent 1px),linear-gradient(#ffffff05 1px,transparent 1px),radial-gradient(ellipse at 30% 40%,#2e6f8950,transparent 38%),radial-gradient(ellipse at 70% 63%,#697a5240,transparent 35%),#172a32;background-size:2rem 2rem,2rem 2rem,auto,auto,auto;box-shadow:inset 0 0 5rem #0008,0 1.5rem 3rem #0005}.table:before,.table:after{content:"";position:absolute;inset:9% 15%;border:1px solid #8fb8c333;border-radius:48% 52% 45% 55%;transform:rotate(9deg);pointer-events:none}.table:after{inset:17% 22%;transform:rotate(-12deg);border-color:#aabb7c35}.realm{position:absolute;inset:18% 19%;display:grid;grid-template-columns:repeat(5,1fr);grid-template-rows:repeat(4,1fr);gap:.38rem;transform:perspective(900px) rotateX(3deg) rotateZ(-1deg)}.cell{position:relative;border:1px solid #6f8d9470;background:#10232b99;box-shadow:inset 0 0 1rem #0007}.cell:nth-child(5n+1),.cell:nth-child(5n+5){background:#23332b99;border-color:#71835f88}.cell:after{content:attr(data-coord);position:absolute;right:.25rem;bottom:.18rem;color:#82969a;font:600 .62rem "Cascadia Mono",monospace}.realm-label{position:absolute;inset:auto 0 8%;text-align:center;color:#9ab0b0;font:700 .72rem "Cascadia Mono",monospace;letter-spacing:.16em;text-transform:uppercase}
    .player{position:absolute;left:1rem;right:1rem;display:grid;grid-template-columns:auto 1fr;gap:.8rem;align-items:center;padding:.7rem .9rem;background:#0c171dd9;border:1px solid #405963;border-radius:.4rem;backdrop-filter:blur(8px)}.player-a{bottom:1rem;border-left:4px solid var(--blue)}.player-b{top:1rem;border-left:4px solid var(--red)}.player h2{font-size:.83rem;letter-spacing:.1em;text-transform:uppercase;margin:0}.player p{margin:.25rem 0 0;color:var(--muted);font: .76rem "Cascadia Mono",monospace}.markers{display:flex;gap:.32rem;justify-content:flex-end;flex-wrap:wrap}.marker{width:2.15rem;height:2.8rem;border:1px solid #869599;border-radius:.18rem;background:repeating-linear-gradient(135deg,#263b44,#263b44 5px,#1b2d34 5px,#1b2d34 10px);display:grid;place-items:center;color:#d9e3dc;font:700 .68rem "Cascadia Mono",monospace}.marker.revealed{background:var(--paper);color:#21333a;border-color:#f9edcf}
    aside{display:grid;gap:1rem;align-content:start}.panel{border:1px solid var(--line);border-radius:.55rem;background:#14242ddd;box-shadow:0 .8rem 2rem #0004;overflow:hidden}.panel h2{font-size:.78rem;letter-spacing:.13em;text-transform:uppercase;margin:0;padding:.72rem .85rem;border-bottom:1px solid var(--line);color:#a8bec1}.panel-body{padding:.8rem}.action-list{display:grid;gap:.55rem}.action{display:grid;text-align:left;gap:.22rem;border-left:3px solid var(--blue)}.action small{color:#9eb0b3;font: .68rem "Cascadia Mono",monospace}.receipt{white-space:pre-wrap;word-break:break-word;max-height:16rem;overflow:auto;margin:0;font: .72rem/1.5 "Cascadia Mono",monospace;color:#b9c9c8}.status{display:grid;grid-template-columns:repeat(3,1fr);gap:.4rem}.datum{padding:.48rem;background:#0c181e;border:1px solid #31464f;border-radius:.3rem;min-width:0}.datum b{display:block;color:#81999d;font-size:.62rem;letter-spacing:.08em;text-transform:uppercase}.datum span{display:block;overflow:hidden;text-overflow:ellipsis;font:700 .7rem "Cascadia Mono",monospace;margin-top:.2rem}.error{border-left:3px solid var(--red);padding:.6rem;color:#f0b0a5}.ok{border-left:3px solid var(--moss);padding:.6rem;color:#c8d5b5}.empty{color:#8fa1a3;font-size:.85rem;margin:0}.footnote{color:#789095;font-size:.72rem;line-height:1.45;margin:.7rem 0 0}
    @media(max-width:850px){header{display:grid}.badges{justify-content:flex-start}.shell{grid-template-columns:1fr}.table{min-height:32rem}.seat-switch{margin-left:0}.realm{inset:22% 8%}.status{grid-template-columns:1fr}}
    @media(prefers-reduced-motion:reduce){*{scroll-behavior:auto!important;transition:none!important}}
  </style>
</head>
<body>
  <header><div><div class="eyebrow">Local deterministic prototype</div><h1>Sorcery Engine Lab</h1></div><div class="badges"><span class="badge warning">Synthetic</span><span class="badge warning">Unranked</span><span class="badge pending">Rules pending</span></div></header>
  <form class="toolbar" id="reset-form"><label>Unsigned 32-bit seed<input id="seed" name="seed" inputmode="numeric" min="0" max="4294967295" step="1" value="1" required></label><button type="submit">Reset session</button><button type="button" id="replay">Verify replay</button><button type="button" id="stale" disabled>Resubmit stale command</button><div class="seat-switch" role="group" aria-label="Observed seat"><button type="button" data-seat="north" aria-pressed="true">North</button><button type="button" data-seat="south" aria-pressed="false">South</button></div></form>
  <main class="shell">
    <section class="table" aria-label="Visual-only five by four realm table">
      <article class="player player-b"><div><h2>South</h2><p id="seat-south-copy">Hidden view</p></div><div class="markers" id="seat-south-markers"></div></article>
      <div class="realm" aria-hidden="true">
        <div class="cell" data-coord="A1"></div><div class="cell" data-coord="B1"></div><div class="cell" data-coord="C1"></div><div class="cell" data-coord="D1"></div><div class="cell" data-coord="E1"></div>
        <div class="cell" data-coord="A2"></div><div class="cell" data-coord="B2"></div><div class="cell" data-coord="C2"></div><div class="cell" data-coord="D2"></div><div class="cell" data-coord="E2"></div>
        <div class="cell" data-coord="A3"></div><div class="cell" data-coord="B3"></div><div class="cell" data-coord="C3"></div><div class="cell" data-coord="D3"></div><div class="cell" data-coord="E3"></div>
        <div class="cell" data-coord="A4"></div><div class="cell" data-coord="B4"></div><div class="cell" data-coord="C4"></div><div class="cell" data-coord="D4"></div><div class="cell" data-coord="E4"></div>
      </div>
      <div class="realm-label">Topographic realm preview · placement and adjacency not implemented</div>
      <article class="player player-a"><div><h2>North</h2><p id="seat-north-copy">Observed view</p></div><div class="markers" id="seat-north-markers"></div></article>
    </section>
    <aside>
      <section class="panel"><h2>Engine state</h2><div class="panel-body status"><div class="datum"><b>Seat</b><span id="current-seat">north</span></div><div class="datum"><b>Version</b><span id="version">—</span></div><div class="datum"><b>State hash</b><span id="hash">—</span></div></div></section>
      <section class="panel"><h2>Engine-issued action dock</h2><div class="panel-body"><div class="action-list" id="actions"><p class="empty">Loading legal actions…</p></div><p class="footnote">The client submits only the displayed action ID, seat, and state version. It computes no rules.</p></div></section>
      <section class="panel"><h2>Event / receipt rail</h2><div class="panel-body"><div id="notice" aria-live="polite"></div><pre class="receipt" id="receipt" aria-live="polite">Starting local session…</pre></div></section>
    </aside>
  </main>
  <script>
    var seat='north', snapshot, lastCommand;
    var byId=function(id){return document.getElementById(id)};
    async function request(path,options){var response=await fetch(path,options);var body=await response.json();if(!response.ok)throw new Error(body.error||('HTTP '+response.status));return body}
    function markerHtml(value,index,hidden){var label=hidden?'?':String(value&&typeof value==='object'?(value.label||value.id||index+1):(value||index+1));return '<span class="marker '+(hidden?'':'revealed')+'" title="'+(hidden?'Hidden marker':label)+'">'+label+'</span>'}
    function renderMarkers(view,owner){var marker=view.markers?.[owner]||{status:'empty'};var hidden=marker.status==='hidden'&&!marker.identity;var html=marker.status==='empty'?'':markerHtml(marker.identity||'?',0,hidden);byId('seat-'+owner+'-markers').innerHTML=html||'<span class="empty">No marker</span>';byId('seat-'+owner+'-copy').textContent=owner===seat?'Observed seat · private identity visible':(marker.status==='revealed'?'Publicly revealed':'Opponent · hidden placeholder only')}
    function render(data){snapshot=data;var view=data.view||data.observation||{};var actions=data.actions||[];byId('current-seat').textContent=seat+(view.activeSeat===seat?' · active':' · observing');byId('version').textContent=String(view.stateVersion??data.stateVersion??'—');byId('hash').textContent=String(data.stateHash||'—');renderMarkers(view,'north');renderMarkers(view,'south');var dock=byId('actions');dock.innerHTML='';if(!actions.length){dock.innerHTML='<p class="empty">No legal actions for this seat.</p>'}actions.forEach(function(action){var button=document.createElement('button');button.type='button';button.className='action';button.dataset.actionId=action.actionId;button.innerHTML='<strong>'+String(action.label)+'</strong><small>'+String(action.actionId)+'</small>';button.addEventListener('click',function(){submit(button.dataset.actionId)});dock.appendChild(button)})}
    async function refresh(){render(await request('/api/view?seat='+seat))}
    async function submit(actionId,command){try{var version=snapshot.view?.stateVersion??snapshot.observation?.stateVersion??snapshot.stateVersion;var next=command||{seat:seat,stateVersion:version,actionId:actionId};if(!command)lastCommand=next;var result=await request('/api/action',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(next)});byId('notice').className=result.accepted?'ok':'error';byId('notice').textContent=result.accepted?'Action accepted':'Rejected: '+(result.reason?.code||result.reason||'unknown');byId('receipt').textContent=JSON.stringify(result.receipt||result.reason||result,null,2);byId('stale').disabled=!lastCommand;render(result)}catch(error){showError(error)}}
    function showError(error){byId('notice').className='error';byId('notice').textContent=error.message}
    document.querySelectorAll('[data-seat]').forEach(function(button){button.addEventListener('click',function(){seat=button.dataset.seat;document.querySelectorAll('[data-seat]').forEach(function(item){item.setAttribute('aria-pressed',String(item===button))});refresh().catch(showError)})});
    byId('reset-form').addEventListener('submit',async function(event){event.preventDefault();try{lastCommand=undefined;byId('stale').disabled=true;var data=await request('/api/reset',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({seed:Number(byId('seed').value)})});byId('notice').className='ok';byId('notice').textContent='Session reset';byId('receipt').textContent=JSON.stringify(data,null,2);render(data)}catch(error){showError(error)}});
    byId('stale').addEventListener('click',function(){if(lastCommand)submit(lastCommand.actionId,lastCommand)});
    byId('replay').addEventListener('click',async function(){try{var data=await request('/api/replay',{method:'POST'});byId('notice').className=data.verified?'ok':'error';byId('notice').textContent=data.verified?'Replay byte-identical':'Replay mismatch';byId('receipt').textContent=JSON.stringify(data,null,2)}catch(error){showError(error)}});
    refresh().catch(showError);
  </script>
</body>
</html>`;

type JsonRecord = Record<string, unknown>;

function isSeat(value: string | null): value is DemoSeat {
  return value === 'north' || value === 'south';
}

async function readJson(request: IncomingMessage): Promise<JsonRecord> {
  const chunks: Buffer[] = [];
  let size = 0;
  for await (const chunk of request) {
    const buffer = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk as Uint8Array);
    size += buffer.length;
    if (size > MAX_BODY_BYTES) throw new Error('request body exceeds 64 KiB');
    chunks.push(buffer);
  }
  const parsed: unknown = JSON.parse(Buffer.concat(chunks).toString('utf8') || '{}');
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

function sendPage(response: ServerResponse): void {
  response.writeHead(200, {
    'cache-control': 'no-store',
    'content-length': Buffer.byteLength(PAGE),
    'content-security-policy': "default-src 'self'; connect-src 'self'; img-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; base-uri 'none'; frame-ancestors 'none'",
    'content-type': 'text/html; charset=utf-8',
    'x-content-type-options': 'nosniff',
  });
  response.end(PAGE);
}

export function createPrototypeServer(initialSeed = 1): Server {
  let manifest: DemoManifest = createDemoManifest(initialSeed);
  let session: DemoSession = createDemoSession(manifest);
  const actionIds: string[] = [];

  function view(seat: DemoSeat): JsonRecord {
    const observation = observeDemo(session.state, seat);
    return {
      actions: legalDemoActions(session.state, seat),
      observation,
      stateHash: hashDemoState(session.state),
      stateVersion: session.state.stateVersion,
      view: observation,
    };
  }

  return createServer(async (request, response) => {
    try {
      const url = new URL(request.url ?? '/', `http://${HOST}`);
      if (request.method === 'GET' && url.pathname === '/') return sendPage(response);
      if (request.method === 'GET' && url.pathname === '/api/view') {
        const seat = url.searchParams.get('seat');
        if (!isSeat(seat)) return sendJson(response, 400, { error: 'seat must be north or south' });
        return sendJson(response, 200, view(seat));
      }
      if (request.method === 'POST' && url.pathname === '/api/reset') {
        const body = await readJson(request);
        if (!Number.isInteger(body.seed) || (body.seed as number) < 0 || (body.seed as number) > 0xffff_ffff) {
          return sendJson(response, 400, { error: 'seed must be an unsigned 32-bit integer' });
        }
        manifest = createDemoManifest(body.seed as number);
        session = createDemoSession(manifest);
        actionIds.length = 0;
        return sendJson(response, 200, view('north'));
      }
      if (request.method === 'POST' && url.pathname === '/api/action') {
        const body = await readJson(request);
        const seat = typeof body.seat === 'string' ? body.seat : null;
        if (!isSeat(seat) ||
            !Number.isInteger(body.stateVersion) || typeof body.actionId !== 'string') {
          return sendJson(response, 400, { error: 'seat, stateVersion, and actionId are required' });
        }
        const command = { seat, stateVersion: body.stateVersion as number, actionId: body.actionId };
        const result = stepDemo(session, command);
        session = result.session;
        if (result.accepted) actionIds.push(command.actionId);
        return sendJson(response, 200, {
          ...view(command.seat),
          accepted: result.accepted,
          ...(result.accepted ? { receipt: result.receipt } : { reason: result.reason }),
        });
      }
      if (request.method === 'POST' && url.pathname === '/api/replay') {
        const replay = replayDemo(manifest, actionIds);
        return sendJson(response, 200, {
          acceptedActionCount: actionIds.length,
          finalStateHash: hashDemoState(replay.state),
          verified: verifyDemoReplay(manifest, actionIds, session.transcript),
        });
      }
      return sendJson(response, 404, { error: 'not found' });
    } catch (error) {
      return sendJson(response, 400, { error: error instanceof Error ? error.message : 'bad request' });
    }
  });
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  createPrototypeServer().listen(DEFAULT_PORT, HOST, () => {
    process.stdout.write(`Sorcery Engine Lab: http://${HOST}:${DEFAULT_PORT}\n`);
  });
}
