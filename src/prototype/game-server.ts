import { createServer, type IncomingMessage, type Server, type ServerResponse } from 'node:http';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  createSyntheticDemoManifest,
  selectDeterministicGameAction,
} from '../commands/run-game-demo.ts';
import {
  createGameManifest,
  createGameSession,
  hashGameState,
  legalGameActions,
  observeGame,
  stepGame,
  verifyGameReplay,
  type GameLegalAction,
  type GameObservation,
  type GameSeat,
  type GameManifest,
  type GameSession,
} from '../engine/game.ts';

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
    :root{color-scheme:dark;--bg:#0d171d;--panel:#14262f;--line:#45606b;--ink:#e1e9e6;--muted:#91a7a9;--north:#3984a1;--south:#bd5949;--site:#718458;--focus:#ffc96b;font-family:Bahnschrift,"Segoe UI",sans-serif;background:var(--bg);color:var(--ink)}*{box-sizing:border-box}body{margin:0;background:radial-gradient(circle at 45% 10%,#294853,transparent 38%),var(--bg);min-height:100vh}button,input,select{font:inherit}button{background:#243d48;border:1px solid #5b7b87;border-radius:.35rem;color:var(--ink);cursor:pointer;padding:.58rem .72rem}button:hover{background:#315362}button:focus-visible,input:focus-visible,select:focus-visible,summary:focus-visible{outline:3px solid var(--focus);outline-offset:2px}button:disabled{opacity:.45;cursor:not-allowed}header,.toolbar{display:flex;gap:.7rem;align-items:center;flex-wrap:wrap;padding:1rem clamp(1rem,4vw,3rem);border-bottom:1px solid var(--line);background:#0c171ddd}header{justify-content:space-between}h1{margin:0;font-size:clamp(1.35rem,3vw,2.15rem);letter-spacing:.06em;text-transform:uppercase}.eyebrow,.badge,label{font-size:.72rem;letter-spacing:.12em;text-transform:uppercase}.eyebrow,label{color:var(--muted)}.badge{border:1px solid #d29b55;border-radius:999px;color:#ffd89c;padding:.35rem .55rem}.toolbar label{display:grid;gap:.25rem}.toolbar input,.toolbar select{background:#091318;border:1px solid #52707b;border-radius:.3rem;color:var(--ink);padding:.55rem}.toolbar input{width:10rem}.toolbar select{max-width:22rem}.seat-switch{display:flex;margin-left:auto}.seat-switch button{border-radius:0}.seat-switch button:first-child{border-radius:.35rem 0 0 .35rem}.seat-switch button:last-child{border-radius:0 .35rem .35rem 0}.seat-switch [aria-pressed=true]{background:#2f7189}.layout{display:grid;grid-template-columns:minmax(0,1fr) minmax(19rem,26rem);gap:1rem;padding:1rem clamp(1rem,4vw,3rem) 2rem;max-width:1600px;margin:auto}.table{position:relative;min-height:43rem;border:1px solid var(--line);border-radius:.65rem;background:#142a33;box-shadow:inset 0 0 5rem #0008}.realm{position:absolute;inset:19% 10%;display:grid;grid-template-columns:repeat(5,1fr);grid-template-rows:repeat(4,1fr);gap:.4rem}.cell{position:relative;border:1px solid #78949b;background:#0d2028;display:grid;place-items:center;align-content:center;gap:.2rem;min-width:0}.cell:after{content:attr(data-cell);position:absolute;right:.2rem;bottom:.15rem;color:#71888e;font:600 .62rem "Cascadia Mono",monospace}.piece{display:grid;gap:.15rem;text-align:center;max-width:95%;font:700 .68rem "Cascadia Mono",monospace}.site{background:#34432c;border:1px solid #879b68;border-radius:.25rem;padding:.35rem}.unit{background:#263f4a;border:1px solid #73a3b4;border-radius:.25rem;padding:.2rem}.avatar{border:2px solid var(--north);border-radius:999px;padding:.2rem .35rem}.avatar.south{border-color:var(--south)}.player{position:absolute;left:1rem;right:1rem;display:grid;grid-template-columns:auto 1fr;gap:.8rem;padding:.65rem .8rem;background:#09151bdc;border:1px solid var(--line);border-radius:.4rem;z-index:2}.player.north{bottom:1rem;border-left:4px solid var(--north)}.player.south{top:1rem;border-left:4px solid var(--south)}.player h2{font-size:.8rem;text-transform:uppercase;margin:0}.player p{color:var(--muted);font:.72rem/1.4 "Cascadia Mono",monospace;margin:.2rem 0}.hand{display:flex;gap:.3rem;flex-wrap:wrap;justify-content:flex-end}.card{border:1px solid #78949b;background:#1c343e;border-radius:.2rem;padding:.28rem;max-width:10rem;overflow:hidden;text-overflow:ellipsis;font:.64rem "Cascadia Mono",monospace}.sidebar{display:grid;gap:1rem;align-content:start}.panel{border:1px solid var(--line);border-radius:.55rem;background:var(--panel);overflow:hidden}.panel h2{font-size:.76rem;letter-spacing:.12em;text-transform:uppercase;margin:0;padding:.7rem .8rem;border-bottom:1px solid var(--line)}.panel-body{padding:.75rem}.status{display:grid;grid-template-columns:repeat(2,1fr);gap:.45rem}.datum{background:#0b1920;border:1px solid #314a54;border-radius:.3rem;padding:.45rem;min-width:0}.datum b{display:block;color:var(--muted);font-size:.62rem;text-transform:uppercase}.datum span{display:block;overflow:hidden;text-overflow:ellipsis;font:700 .72rem "Cascadia Mono",monospace;margin-top:.18rem}.actions{display:grid;gap:.45rem;max-height:20rem;overflow:auto}.action{text-align:left;border-left:3px solid var(--north)}details{border-top:1px solid var(--line);margin-top:.55rem;padding-top:.55rem}summary{cursor:pointer;color:#c6d3d2}.receipt{white-space:pre-wrap;word-break:break-word;max-height:13rem;overflow:auto;margin:0;font:.7rem/1.45 "Cascadia Mono",monospace;color:#bed0ce}.ok{border-left:3px solid var(--site);padding:.5rem}.error{border-left:3px solid var(--south);padding:.5rem}.empty{color:var(--muted);font-size:.82rem}.footnote{color:#81979a;font-size:.72rem;line-height:1.45}.hidden{display:none}@media(max-width:900px){.layout{grid-template-columns:1fr}.table{min-height:38rem}.realm{inset:22% 4%}.seat-switch{margin-left:0}}@media(prefers-reduced-motion:reduce){*{transition:none!important}}
  </style>
</head>
<body>
  <header><div><div class="eyebrow">Authoritative rules checkpoint</div><h1>Sorcery Playable Core</h1></div><span class="badge" id="mode">Unranked · partial rules</span></header>
  <form class="toolbar" id="reset-form"><label>Starter matchup<select id="preset" aria-label="Starter matchup"></select></label><label>Opponent<select id="opponent" aria-label="Opponent"><option value="south">South computer</option><option value="manual">Hot seat</option></select></label><label>Seed<input id="seed" inputmode="numeric" min="0" max="4294967295" step="1" value="1" required></label><button>Reset match</button><button type="button" id="replay">Verify replay</button><button type="button" id="stale" disabled>Resubmit stale</button><div class="seat-switch" role="group" aria-label="Observed seat"><button type="button" data-seat="north" aria-pressed="true">North</button><button type="button" data-seat="south" aria-pressed="false">South</button></div></form>
  <main class="layout">
    <section class="table" aria-label="Five by four realm">
      <article class="player south"><div><h2>South</h2><p id="south-stats"></p></div><div class="hand" id="south-hand"></div></article>
      <div class="realm" id="realm">${CELLS}</div>
      <article class="player north"><div><h2>North</h2><p id="north-stats"></p></div><div class="hand" id="north-hand"></div></article>
    </section>
    <aside class="sidebar">
      <section class="panel"><h2>Match state</h2><div class="panel-body status"><div class="datum"><b>Observer</b><span id="observer">north</span></div><div class="datum"><b>Active</b><span id="active">—</span></div><div class="datum"><b>Turn / phase</b><span id="phase">—</span></div><div class="datum"><b>Version</b><span id="version">—</span></div><div class="datum" style="grid-column:1/-1"><b>State hash</b><span id="hash">—</span></div></div></section>
      <section class="panel"><h2>Engine-issued actions</h2><div class="panel-body"><div class="actions" id="actions"><p class="empty">Loading…</p></div><p class="footnote">During mulligan, “Keep opening hand” is the simplest path. Every alternative below is a fully bound legal choice.</p></div></section>
      <section class="panel"><h2>Receipt / events</h2><div class="panel-body"><div id="notice" aria-live="polite"></div><pre id="receipt" class="receipt" aria-live="polite">Starting match…</pre></div></section>
    </aside>
  </main>
  <script>
    var seat='north',snapshot,lastCommand;var byId=function(id){return document.getElementById(id)};
    async function request(path,options){var response=await fetch(path,options);var body=await response.json();if(!response.ok)throw new Error(body.error||('HTTP '+response.status));return body}
    function escapeHtml(value){return String(value).replace(/[&<>"']/g,function(character){return {'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[character]})}
    function cardName(cardId){return snapshot&&snapshot.cardNames&&snapshot.cardNames[cardId]||cardId}
    function displayText(value){var result=String(value);Object.entries(snapshot&&snapshot.cardNames||{}).forEach(function(entry){result=result.replaceAll(entry[0],entry[1])});return result}
    function cardChip(card){var id=escapeHtml(card.cardId),name=escapeHtml(cardName(card.cardId));return '<span class="card" title="'+id+'">'+name+'</span>'}
    function renderPlayer(view,owner){var player=view.players[owner],hand=player.hand,affinity=player.affinity;byId(owner+'-stats').textContent='Life '+player.avatar.life+' · Atlas '+player.atlasCount+' · Spellbook '+player.spellbookCount+' · Cemetery '+player.cemetery.length+' · Mana '+player.mana+' · Affinity E'+affinity.earth+' F'+affinity.fire+' W'+affinity.water+' A'+affinity.air+(player.domainEstablished?' · Domain established':' · Domain pending');var cards=[];['atlas','spellbook'].forEach(function(zone){var value=hand[zone];if(Array.isArray(value)){value.forEach(function(card){cards.push(cardChip(card))})}else{cards.push('<span class="card">'+value+' hidden '+zone+'</span>')}});byId(owner+'-hand').innerHTML=cards.join('')}
    function renderRealm(view){document.querySelectorAll('[data-cell]').forEach(function(cell){cell.innerHTML=''});Object.entries(view.realm.sites||{}).forEach(function(entry){var cell=document.querySelector('[data-cell="'+entry[0]+'"]');if(cell)cell.innerHTML+='<span class="piece site">'+escapeHtml(cardName(entry[1].cardId))+'</span>'});(view.realm.units||[]).forEach(function(unit){var cell=document.querySelector('[data-cell="'+unit.location+'"]');if(cell)cell.innerHTML+='<span class="piece unit">'+escapeHtml(cardName(unit.cardId))+' · '+unit.attack+'/'+unit.defense+(unit.damage?' · '+unit.damage+' dmg':'')+(unit.tapped?' · tapped':'')+(unit.summoningSickness?' · new':'')+'</span>'});['north','south'].forEach(function(owner){var avatar=view.players[owner].avatar,cell=document.querySelector('[data-cell="'+avatar.location+'"]');if(cell)cell.innerHTML+='<span class="piece avatar '+owner+'">'+owner+' avatar · '+avatar.attack+' atk · '+avatar.life+' life'+(avatar.tapped?' · tapped':'')+'</span>'})}
    function actionButton(candidate){var button=document.createElement('button');button.type='button';button.className='action';button.textContent=displayText(candidate.label);button.title=JSON.stringify(candidate.descriptor);button.addEventListener('click',function(){submit(candidate.actionId)});return button}
    function renderActions(actions,view){var dock=byId('actions');dock.innerHTML='';if(view.terminal.status==='finished'){var terminal=view.terminal,outcome=terminal.result==='draw'?'Draw':'Winner: '+escapeHtml(terminal.winner)+' · Loser: '+escapeHtml(terminal.loser);dock.innerHTML='<div class="ok" role="status" aria-live="polite"><strong>Game over</strong><p>'+outcome+' · Reason: '+escapeHtml(terminal.reason.replaceAll('_',' '))+'</p></div>';return}if(!actions.length){dock.innerHTML='<p class="empty">No legal actions for this observer.</p>';return}if(view.phase==='mulligan'){var keep=actions.filter(function(a){return a.descriptor.kind==='mulligan'&&!a.descriptor.atlasOrder.length&&!a.descriptor.spellbookOrder.length});keep.forEach(function(a){dock.appendChild(actionButton(a))});var rest=actions.filter(function(a){return keep.indexOf(a)<0});var details=document.createElement('details'),summary=document.createElement('summary'),list=document.createElement('div');summary.textContent='Show '+rest.length+' mulligan alternatives';list.className='actions';rest.forEach(function(a){list.appendChild(actionButton(a))});details.append(summary,list);dock.appendChild(details);return}actions.forEach(function(a){dock.appendChild(actionButton(a))})}
    function render(data){snapshot=data;var view=data.view,picker=byId('preset');if(picker.options.length!==data.presets.length){picker.innerHTML=data.presets.map(function(preset){return '<option value="'+escapeHtml(preset.id)+'">'+escapeHtml(preset.label)+'</option>'}).join('')}picker.value=data.presetId;byId('opponent').value=data.opponent;byId('seed').value=String(data.seed);byId('mode').textContent=(data.mode==='private-local'?'Private-local actual cards · unranked':'Synthetic fallback · unranked')+(data.opponent==='south'?' · vs computer':' · hot seat');byId('observer').textContent=seat;byId('active').textContent=view.activeSeat+(view.decisionSeat===view.activeSeat?'':' · '+view.decisionSeat+' deciding');byId('phase').textContent='Turn '+view.turnNumber+' · '+view.phase;byId('version').textContent=view.stateVersion;byId('hash').textContent=data.stateHash;renderPlayer(view,'north');renderPlayer(view,'south');renderRealm(view);renderActions(data.actions,view);syncSeatButtons()}
    function syncSeatButtons(){document.querySelectorAll('[data-seat]').forEach(function(button){button.disabled=Boolean(snapshot&&snapshot.opponent==='south'&&button.dataset.seat==='south');button.setAttribute('aria-pressed',String(button.dataset.seat===seat))})}
    async function refresh(){render(await request('/api/view?seat='+seat))}
    async function submit(actionId,command){try{var next=command||{actionId:actionId,seat:seat,stateVersion:snapshot.view.stateVersion};if(!command)lastCommand=next;var result=await request('/api/action',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(next)});byId('notice').className=result.accepted?'ok':'error';byId('notice').textContent=result.accepted?'Action accepted'+(result.opponentActionCount?' · South computer took '+result.opponentActionCount+' action'+(result.opponentActionCount===1?'':'s'):''):'Rejected: '+result.reason.code;byId('receipt').textContent=JSON.stringify(result.receipt||result.reason,null,2);render(result);byId('stale').disabled=!lastCommand;if(result.accepted&&result.opponent==='manual'&&result.view.decisionSeat!==seat){seat=result.view.decisionSeat;syncSeatButtons();await refresh()}}catch(error){showError(error)}}
    function showError(error){byId('notice').className='error';byId('notice').textContent=error.message}
    document.querySelectorAll('[data-seat]').forEach(function(button){button.addEventListener('click',function(){if(button.disabled)return;seat=button.dataset.seat;syncSeatButtons();refresh().catch(showError)})});
    byId('preset').addEventListener('change',function(){var selected=snapshot.presets.find(function(preset){return preset.id===byId('preset').value});if(selected)byId('seed').value=String(selected.seed)});
    byId('reset-form').addEventListener('submit',async function(event){event.preventDefault();try{seat='north';lastCommand=undefined;syncSeatButtons();byId('stale').disabled=true;var data=await request('/api/reset',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({opponent:byId('opponent').value,presetId:byId('preset').value,seed:Number(byId('seed').value)})});byId('notice').className='ok';byId('notice').textContent='Match reset';byId('receipt').textContent=JSON.stringify({stateHash:data.stateHash},null,2);render(data)}catch(error){showError(error)}});
    byId('stale').addEventListener('click',function(){if(lastCommand)submit(lastCommand.actionId,lastCommand)});
    byId('replay').addEventListener('click',async function(){try{var data=await request('/api/replay',{method:'POST'});byId('notice').className=data.verified?'ok':'error';byId('notice').textContent=data.verified?'Replay byte-identical':'Replay mismatch';byId('receipt').textContent=JSON.stringify(data,null,2)}catch(error){showError(error)}});
    refresh().catch(showError);
  </script>
</body>
</html>`;

type JsonRecord = Record<string, unknown>;

function isSeat(value: string | null): value is GameSeat {
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

function displayActionLabel(
  action: GameLegalAction,
  observation: GameObservation,
  cardNames: Readonly<Record<string, string>>,
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
  return label;
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
  let session: GameSession = createGameSession(reseedManifest(
    selectedPreset.manifest,
    initialSeed ?? selectedPreset.manifest.seed,
  ));

  function advanceOpponent(start: GameSession): Readonly<{ count: number; session: GameSession }> {
    let next = start;
    let count = 0;
    while (next.state.terminal.status === 'active' && next.state.decisionSeat === 'south') {
      if (count >= MAX_OPPONENT_ACTIONS) throw new Error('deterministic opponent exceeded action limit');
      const result = stepGame(next, selectDeterministicGameAction(next));
      if (!result.accepted) throw new Error(`deterministic opponent action rejected: ${result.reason.code}`);
      next = result.session;
      count += 1;
    }
    return { count, session: next };
  }

  function view(seat: GameSeat): JsonRecord {
    const observation = observeGame(session.state, seat);
    const visibleObservation = JSON.stringify(observation);
    const cardNames = Object.fromEntries(Object.entries(selectedPreset.cardNames ?? {})
      .filter(([cardId]) => visibleObservation.includes(JSON.stringify(cardId))));
    return {
      actions: legalGameActions(session.state, seat)
        .map((action) => ({ ...action, label: displayActionLabel(action, observation, cardNames) })),
      cardNames,
      mode: session.manifest.authority.mode,
      opponent,
      presetId: selectedPreset.id,
      presets: presets.map(({ id, label, manifest }) => ({ id, label, seed: manifest.seed })),
      seed: session.manifest.seed,
      stateHash: hashGameState(session.state),
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
        if (opponent === 'south' && seat === 'south') {
          return sendJson(response, 403, { error: 'south is hidden while controlled by the deterministic opponent' });
        }
        return sendJson(response, 200, view(seat));
      }
      if (request.method === 'POST' && url.pathname === '/api/reset') {
        const body = await readJson(request);
        if (!Number.isInteger(body.seed) || (body.seed as number) < 0 || (body.seed as number) > 0xffff_ffff) {
          return sendJson(response, 400, { error: 'seed must be an unsigned 32-bit integer' });
        }
        const presetId = body.presetId === undefined ? selectedPreset.id : body.presetId;
        const preset = typeof presetId === 'string'
          ? presets.find(({ id }) => id === presetId)
          : undefined;
        if (!preset) return sendJson(response, 400, { error: 'presetId must name an available preset' });
        const requestedOpponent = body.opponent === undefined ? opponent : body.opponent;
        if (requestedOpponent !== 'manual' && requestedOpponent !== 'south') {
          return sendJson(response, 400, { error: 'opponent must be manual or south' });
        }
        selectedPreset = preset;
        opponent = requestedOpponent;
        session = createGameSession(reseedManifest(selectedPreset.manifest, body.seed as number));
        return sendJson(response, 200, view('north'));
      }
      if (request.method === 'POST' && url.pathname === '/api/action') {
        const body = await readJson(request);
        const seat = typeof body.seat === 'string' ? body.seat : null;
        if (!isSeat(seat)
          || !Number.isSafeInteger(body.stateVersion)
          || (body.stateVersion as number) < 0
          || typeof body.actionId !== 'string'
          || body.actionId.length > 128) {
          return sendJson(response, 400, { error: 'valid seat, stateVersion, and actionId are required' });
        }
        if (opponent === 'south' && seat === 'south') {
          return sendJson(response, 400, { error: 'south is controlled by the deterministic opponent' });
        }
        const result = stepGame(session, {
          actionId: body.actionId,
          seat,
          stateVersion: body.stateVersion as number,
        });
        const advanced = result.accepted && opponent === 'south'
          ? advanceOpponent(result.session)
          : { count: 0, session: result.session };
        session = advanced.session;
        return sendJson(response, 200, {
          ...view(seat),
          accepted: result.accepted,
          opponentActionCount: advanced.count,
          ...(result.accepted ? { receipt: result.receipt } : { reason: result.reason }),
        });
      }
      if (request.method === 'POST' && url.pathname === '/api/replay') {
        return sendJson(response, 200, {
          acceptedActionCount: session.transcript.length,
          finalStateHash: hashGameState(session.state),
          verified: verifyGameReplay(session),
        });
      }
      return sendJson(response, 404, { error: 'not found' });
    } catch (error) {
      return sendJson(response, 400, { error: error instanceof Error ? error.message : 'bad request' });
    }
  });
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
