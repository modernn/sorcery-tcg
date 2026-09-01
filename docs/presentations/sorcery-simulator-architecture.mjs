import fs from "node:fs/promises";
import path from "node:path";
import { Presentation, PresentationFile } from "@oai/artifact-tool";

const [pptxPath, previewPath, renderDir] = process.argv.slice(2);
if (!pptxPath || !previewPath || !renderDir) {
  throw new Error("usage: build.mjs <deck.pptx> <preview.webp> <render-dir>");
}

const C = {
  canvas: "#FFFFFF",
  ink: "#000000",
  muted: "#5B6068",
  panel: "#EDEDED",
  rule: "#B8BCC4",
  accent: "#6DCBF4",
  accentStrong: "#3D8DFF",
  accentPale: "#D0EDFA",
};
const FONT = "Helvetica Neue";
const presentation = Presentation.create({ slideSize: { width: 1280, height: 720 } });

function addText(slide, text, position, options = {}) {
  const shape = slide.shapes.add({
    geometry: "textbox",
    name: options.name,
    position,
    fill: "none",
    line: { style: "solid", fill: "none", width: 0 },
  });
  shape.text = text;
  shape.text.style = {
    typeface: FONT,
    fontSize: options.fontSize ?? 20,
    bold: options.bold ?? false,
    color: options.color ?? C.ink,
    alignment: options.alignment ?? "left",
    verticalAlignment: options.verticalAlignment ?? "top",
    autoFit: options.autoFit ?? "shrinkText",
    insets: options.insets ?? { top: 0, right: 0, bottom: 0, left: 0 },
  };
  return shape;
}

function addBox(slide, position, options = {}) {
  return slide.shapes.add({
    geometry: options.geometry ?? "rect",
    name: options.name,
    position,
    fill: options.fill ?? C.panel,
    line: {
      style: "solid",
      fill: options.lineFill ?? "none",
      width: options.lineWidth ?? 0,
    },
  });
}

function addRule(slide, left, top, width, color = C.rule, weight = 1) {
  return slide.shapes.add({
    geometry: "straightConnector1",
    position: { left, top, width, height: 0 },
    fill: "none",
    line: { style: "solid", fill: color, width: weight },
  });
}

function addArrow(slide, left, top, width = 42) {
  addText(slide, "→", { left, top, width, height: 42 }, {
    fontSize: 34,
    bold: true,
    color: C.accentStrong,
    alignment: "center",
    verticalAlignment: "middle",
  });
}

function addTitle(slide, title, page) {
  addText(slide, title, { left: 42, top: 34, width: 1160, height: 76 }, {
    name: `slide-${page}-title`,
    fontSize: 42,
    bold: true,
  });
  addText(slide, String(page).padStart(2, "0"), { left: 1185, top: 660, width: 52, height: 22 }, {
    fontSize: 14,
    color: C.muted,
    alignment: "right",
  });
}

function labelBox(slide, label, body, position, options = {}) {
  addBox(slide, position, {
    fill: options.fill ?? C.panel,
    lineFill: options.lineFill,
    lineWidth: options.lineWidth,
  });
  addText(slide, label, {
    left: position.left + 22,
    top: position.top + 18,
    width: position.width - 44,
    height: 34,
  }, { fontSize: options.labelSize ?? 22, bold: true, color: options.labelColor ?? C.ink });
  addText(slide, body, {
    left: position.left + 22,
    top: position.top + 60,
    width: position.width - 44,
    height: position.height - 78,
  }, { fontSize: options.bodySize ?? 18, color: options.bodyColor ?? C.muted });
}

function notes(slide, lines, sources) {
  slide.speakerNotes.textFrame.setText([
    ...lines,
    "",
    "[Sources]",
    ...sources.map((source) => `- ${source}`),
  ]);
}

function newSlide() {
  const slide = presentation.slides.add();
  slide.background.fill = C.canvas;
  return slide;
}

// 1 — Cover: Codex Grid slide-01 hierarchy.
{
  const slide = newSlide();
  addText(slide, "SORCERY SIMULATOR", { left: 42, top: 42, width: 420, height: 32 }, {
    fontSize: 20,
    bold: true,
    color: C.accentStrong,
  });
  addText(slide, "One authoritative engine\nfor rules, simulation,\nand deck search", {
    left: 42,
    top: 176,
    width: 1040,
    height: 290,
  }, { fontSize: 76, bold: true });
  addRule(slide, 42, 486, 1196, C.accentStrong, 4);
  addText(slide, "How Rust, private card data, deterministic self-play, and LLM tools fit together", {
    left: 42,
    top: 520,
    width: 850,
    height: 78,
  }, { fontSize: 24, color: C.muted });
  addText(slide, "ARCHITECTURE OVERVIEW", { left: 930, top: 612, width: 308, height: 28 }, {
    fontSize: 16,
    bold: true,
    color: C.muted,
    alignment: "right",
  });
  notes(slide, ["The deck separates the authoritative rules path from research, search, and explanation."], [
    "C:\\src\\sorcery-tcg\\AGENTS.md",
    "C:\\src\\sorcery-tcg\\docs\\rules-catalog.md",
  ]);
}

// 2 — System boundary.
{
  const slide = newSlide();
  addTitle(slide, "Rust owns every move; everything else asks", 2);
  addRule(slide, 42, 132, 1196);
  labelBox(slide, "TypeScript boundary", "Authority ingestion\nServer transport\nBrowser-facing payloads", {
    left: 42, top: 206, width: 292, height: 238,
  });
  addArrow(slide, 348, 296);
  labelBox(slide, "Authoritative Rust", "RulesContext + state\nLegal actions + transitions\nCheckpoints + simulation\nSearch + final replay", {
    left: 406, top: 176, width: 424, height: 300,
  }, { fill: C.accentPale, lineFill: C.accentStrong, lineWidth: 2, labelSize: 28, bodySize: 21 });
  addArrow(slide, 846, 296);
  labelBox(slide, "Consumers", "CLI and server\nDeck optimizer\nMCP / LLM tools\nHuman-readable reports", {
    left: 904, top: 206, width: 334, height: 238,
  });
  addText(slide, "Only engine-issued actions cross into state changes.", {
    left: 406, top: 520, width: 424, height: 46,
  }, { fontSize: 22, bold: true, color: C.accentStrong, alignment: "center" });
  addText(slide, "The UI can change later without changing what is legal.", {
    left: 42, top: 602, width: 1196, height: 36,
  }, { fontSize: 20, color: C.muted, alignment: "center" });
  notes(slide, ["The engine is a service boundary even when it is linked as a native process rather than exposed over a network."], [
    "C:\\src\\sorcery-tcg\\AGENTS.md",
    "C:\\src\\sorcery-tcg\\crates\\sorcery-engine\\src\\lib.rs",
  ]);
}

// 3 — Rules and state.
{
  const slide = newSlide();
  addTitle(slide, "Readable facts become typed legal actions", 3);
  addRule(slide, 42, 132, 1196);
  const y = 230;
  const w = 236;
  labelBox(slide, "RulesContext", "Immutable card and rule facts loaded once", { left: 42, top: y, width: w, height: 190 }, { fill: C.accentPale });
  addArrow(slide, 284, 300, 34);
  labelBox(slide, "GameState", "Compact IDs, enums, arrays, zones, PRNG", { left: 324, top: y, width: w, height: 190 });
  addArrow(slide, 566, 300, 34);
  labelBox(slide, "LegalAction", "Canonical ordered choices issued by the engine", { left: 606, top: y, width: w, height: 190 }, { fill: C.accentPale });
  addArrow(slide, 848, 300, 34);
  labelBox(slide, "Transition", "Validate action ID, mutate state, emit events", { left: 888, top: y, width: 350, height: 190 });
  addText(slide, "Card names do not decide legality.", { left: 42, top: 496, width: 470, height: 40 }, {
    fontSize: 28, bold: true,
  });
  addText(slide, "Generic facts route through shared helpers; the public rules catalog points to one direct scenario proof per supported behavior.", {
    left: 42, top: 550, width: 1110, height: 72,
  }, { fontSize: 21, color: C.muted });
  notes(slide, ["RulesContext is shared across positions. A legal action is a typed choice, not a client-authored mutation."], [
    "C:\\src\\sorcery-tcg\\docs\\rules-catalog.md",
    "C:\\src\\sorcery-tcg\\crates\\sorcery-engine\\src\\facts.rs",
    "C:\\src\\sorcery-tcg\\crates\\sorcery-engine\\src\\action.rs",
    "C:\\src\\sorcery-tcg\\crates\\sorcery-engine\\src\\game.rs",
  ]);
}

// 4 — Search vs replay.
{
  const slide = newSlide();
  addTitle(slide, "Search stays light; the chosen line gets full proof", 4);
  addRule(slide, 42, 132, 1196);
  addText(slide, "MANY SPECULATIVE BRANCHES", { left: 42, top: 176, width: 370, height: 28 }, {
    fontSize: 18, bold: true, color: C.accentStrong,
  });
  const top = 232;
  labelBox(slide, "Checkpoint", "Deterministic branch root", { left: 42, top, width: 214, height: 132 }, { fill: C.accentPale });
  addArrow(slide, 266, 273);
  labelBox(slide, "Clone position", "Compact in-memory copy", { left: 318, top, width: 214, height: 132 });
  addArrow(slide, 542, 273);
  labelBox(slide, "Batch rollouts", "Stay inside native Rust", { left: 594, top, width: 236, height: 132 }, { fill: C.accentPale });
  addArrow(slide, 840, 273);
  labelBox(slide, "Score", "Strength, cost, risk", { left: 892, top, width: 214, height: 132 });
  addText(slide, "No JSON, hashing, journals, or FFI crossing at each node", { left: 42, top: 382, width: 1064, height: 34 }, {
    fontSize: 19, color: C.muted,
  });
  addText(slide, "ONE SELECTED RESULT", { left: 42, top: 456, width: 310, height: 28 }, {
    fontSize: 18, bold: true, color: C.accentStrong,
  });
  addRule(slide, 42, 522, 1196, C.accent, 6);
  addText(slide, "manifest + seed", { left: 42, top: 540, width: 190, height: 32 }, { fontSize: 21, bold: true });
  addText(slide, "→", { left: 236, top: 532, width: 40, height: 42 }, { fontSize: 32, bold: true, color: C.accentStrong });
  addText(slide, "authoritative replay", { left: 284, top: 540, width: 220, height: 32 }, { fontSize: 21, bold: true });
  addText(slide, "→", { left: 516, top: 532, width: 40, height: 42 }, { fontSize: 32, bold: true, color: C.accentStrong });
  addText(slide, "events + hashes + receipts + journal", { left: 564, top: 540, width: 430, height: 32 }, { fontSize: 21, bold: true });
  notes(slide, ["Rollouts optimize choice. The authoritative replay proves the selected result and remains reproducible from its manifest and seed."], [
    "C:\\src\\sorcery-tcg\\AGENTS.md",
    "C:\\src\\sorcery-tcg\\crates\\sorcery-engine\\src\\checkpoint.rs",
    "C:\\src\\sorcery-tcg\\crates\\sorcery-engine\\src\\simulator.rs",
    "C:\\src\\sorcery-tcg\\crates\\sorcery-engine\\src\\canonical.rs",
  ]);
}

// 5 — Data boundaries.
{
  const slide = newSlide();
  addTitle(slide, "The catalog supports research—not runtime legality", 5);
  addRule(slide, 42, 132, 1196);
  addBox(slide, { left: 42, top: 186, width: 374, height: 338 }, { fill: C.panel });
  addText(slide, "IGNORED PRIVATE BOUNDARY", { left: 66, top: 210, width: 300, height: 28 }, {
    fontSize: 17, bold: true, color: C.muted,
  });
  addText(slide, ".local/authority/", { left: 66, top: 258, width: 300, height: 38 }, { fontSize: 28, bold: true });
  addText(slide, "Immutable SQLite catalog\n• normalized card facts\n• source-qualified IDs\n• printing aliases\n• price snapshots and quotes", {
    left: 66, top: 320, width: 300, height: 160,
  }, { fontSize: 19, color: C.muted });
  addArrow(slide, 428, 320, 46);
  labelBox(slide, "Load once", "Validate manifest and materialize shared immutable facts", {
    left: 490, top: 244, width: 260, height: 190,
  }, { fill: C.accentPale });
  addArrow(slide, 764, 320, 46);
  labelBox(slide, "In-memory Rust", "RulesContext\nCompact positions\nWhole rollout batches", {
    left: 826, top: 210, width: 412, height: 258,
  }, { fill: C.accentPale, lineFill: C.accentStrong, lineWidth: 2, labelSize: 28, bodySize: 21 });
  addText(slide, "Research path", { left: 42, top: 572, width: 170, height: 30 }, { fontSize: 20, bold: true });
  addText(slide, "catalog → deck corpus + prices → candidate generator → simulation", {
    left: 222, top: 572, width: 830, height: 34,
  }, { fontSize: 22, color: C.accentStrong });
  addText(slide, "A database or vector store enters only when measured research workloads need it.", {
    left: 42, top: 622, width: 1100, height: 32,
  }, { fontSize: 18, color: C.muted });
  notes(slide, ["SQLite is rebuildable and private. The rollout hot path never queries it."], [
    "C:\\src\\sorcery-tcg\\docs\\local-card-catalog.md",
    "C:\\src\\sorcery-tcg\\src\\catalog\\card-catalog.ts",
    "C:\\src\\sorcery-tcg\\docs\\rules-catalog.md",
  ]);
}

// 6 — Price-power optimization.
{
  const slide = newSlide();
  addTitle(slide, "Deck search keeps the strongest affordable frontier", 6);
  addRule(slide, 42, 132, 1196);
  addText(slide, "CONCEPTUAL PRICE–POWER FRONTIER", { left: 42, top: 166, width: 470, height: 28 }, {
    fontSize: 17, bold: true, color: C.muted,
  });
  addRule(slide, 94, 578, 482, C.ink, 2);
  slide.shapes.add({
    geometry: "straightConnector1",
    position: { left: 94, top: 224, width: 0, height: 354 },
    fill: "none",
    line: { style: "solid", fill: C.ink, width: 2 },
  });
  addText(slide, "lower price →", { left: 214, top: 592, width: 250, height: 28 }, { fontSize: 18, color: C.muted, alignment: "center" });
  addText(slide, "more wins →", { left: 28, top: 332, width: 130, height: 28 }, { fontSize: 18, color: C.muted, alignment: "center" });
  const points = [
    [168, 500, C.rule], [234, 448, C.rule], [310, 478, C.rule], [380, 382, C.accentStrong],
    [454, 326, C.accentStrong], [510, 264, C.accentStrong], [286, 350, C.rule], [420, 460, C.rule],
  ];
  for (const [left, top, fill] of points) {
    addBox(slide, { left, top, width: 18, height: 18 }, { geometry: "ellipse", fill });
  }
  addRule(slide, 385, 390, 76, C.accentStrong, 3);
  addRule(slide, 461, 334, 58, C.accentStrong, 3);
  addText(slide, "Keep decks no other candidate beats\non both cost and replay-verified strength.", {
    left: 112, top: 236, width: 410, height: 74,
  }, { fontSize: 19, bold: true, color: C.accentStrong });
  labelBox(slide, "1  Build the corpus", "Import permissioned deck lists and preserve provenance.", {
    left: 664, top: 184, width: 574, height: 126,
  });
  labelBox(slide, "2  Price exactly", "Use printing, variant, condition, currency, and observation time.", {
    left: 664, top: 330, width: 574, height: 126,
  }, { fill: C.accentPale });
  labelBox(slide, "3  Compare fairly", "Rust compare_decks fixes policy, seeds, opponents, and seats; it replays only the winner.", {
    left: 664, top: 476, width: 574, height: 126,
  });
  notes(slide, ["The plot is conceptual, not benchmark data. Price is an objective for deck search, never a rule in game state."], [
    "C:\\src\\sorcery-tcg\\docs\\self-play-reliability.md",
    "C:\\src\\sorcery-tcg\\docs\\local-card-catalog.md",
    "C:\\src\\sorcery-tcg\\crates\\sorcery-engine\\src\\deck.rs",
    "C:\\src\\sorcery-tcg\\crates\\sorcery-engine\\src\\selfplay.rs",
  ]);
}

// 7 — Self-play loop, based on Codex Grid process hierarchy.
{
  const slide = newSlide();
  addTitle(slide, "Self-play improves decisions, never legality", 7);
  addRule(slide, 42, 132, 1196);
  const xs = [42, 284, 526, 768, 1010];
  const titles = ["Freeze inputs", "Make neighbors", "Run paired games", "Promote carefully", "Audit + seal"];
  const bodies = [
    "Authority, deck, opponents, policy, seed partitions",
    "One-step deterministic policy changes",
    "Both seats on the same held-out seeds",
    "Statistical and subgroup gates must pass",
    "Fresh authoritative replays and checkpoint",
  ];
  for (let index = 0; index < xs.length; index += 1) {
    if (index < xs.length - 1) addRule(slide, xs[index] + 116, 310, 126, C.rule, 2);
  }
  for (let index = 0; index < xs.length; index += 1) {
    addBox(slide, { left: xs[index], top: 286, width: 34, height: 34 }, {
      geometry: "ellipse",
      fill: index === 4 ? C.accentStrong : C.accent,
    });
    addText(slide, String(index + 1), { left: xs[index], top: 291, width: 34, height: 22 }, {
      fontSize: 16, bold: true, alignment: "center", color: index === 4 ? C.canvas : C.ink,
    });
    addText(slide, titles[index], { left: xs[index], top: 350, width: 194, height: 32 }, { fontSize: 22, bold: true });
    addText(slide, bodies[index], { left: xs[index], top: 396, width: 194, height: 106 }, { fontSize: 17, color: C.muted });
  }
  addBox(slide, { left: 42, top: 560, width: 1196, height: 64 }, { fill: C.accentPale });
  addText(slide, "Change one axis per experiment: deck composition or policy—not both.", {
    left: 70, top: 579, width: 1140, height: 30,
  }, { fontSize: 23, bold: true, alignment: "center" });
  notes(slide, ["The loop can improve policy choices only after the exercised rules are supported and replay-verifiable."], [
    "C:\\src\\sorcery-tcg\\docs\\self-play-reliability.md",
    "C:\\src\\sorcery-tcg\\crates\\sorcery-engine\\src\\selfplay.rs",
    "C:\\src\\sorcery-tcg\\crates\\sorcery-engine\\tests\\selfplay.rs",
  ]);
}

// 8 — Verification gates.
{
  const slide = newSlide();
  addTitle(slide, "Every rules bug becomes a permanent scenario", 8);
  addRule(slide, 42, 132, 1196);
  addText(slide, "81 / 160", { left: 42, top: 178, width: 350, height: 100 }, { fontSize: 72, bold: true, color: C.accentStrong });
  addText(slide, "public catalog scenarios\nproved directly in Rust today", {
    left: 46, top: 286, width: 320, height: 76,
  }, { fontSize: 22, color: C.muted });
  addText(slide, "Coverage is explicit: unsupported exercised mechanics fail closed instead of silently becoming valid training data.", {
    left: 42, top: 410, width: 340, height: 126,
  }, { fontSize: 20, bold: true });
  const gates = [
    ["Direct scenario proof", "One public test for each supported rule slice"],
    ["Locked Rust gates", "format • check • Clippy • workspace tests"],
    ["Parity + determinism", "action order • state • events • replay hashes"],
    ["Release evidence", "paired seat swaps • throughput • memory"],
    ["Private authority", "only when ignored local inputs exist"],
  ];
  for (let index = 0; index < gates.length; index += 1) {
    const top = 174 + index * 92;
    addBox(slide, { left: 462, top, width: 50, height: 50 }, { fill: index < 3 ? C.accent : C.panel });
    addText(slide, String(index + 1), { left: 462, top: top + 11, width: 50, height: 24 }, {
      fontSize: 18, bold: true, alignment: "center",
    });
    addText(slide, gates[index][0], { left: 542, top: top - 2, width: 300, height: 30 }, { fontSize: 23, bold: true });
    addText(slide, gates[index][1], { left: 542, top: top + 36, width: 600, height: 30 }, { fontSize: 18, color: C.muted });
  }
  notes(slide, ["Catalog count taken from data/rules/catalog.json at deck generation time: 81 rust-supported and 79 typescript-supported entries."], [
    "C:\\src\\sorcery-tcg\\data\\rules\\catalog.json",
    "C:\\src\\sorcery-tcg\\docs\\rules-catalog.md",
    "C:\\src\\sorcery-tcg\\AGENTS.md",
  ]);
}

// 9 — LLM/MCP boundary.
{
  const slide = newSlide();
  addTitle(slide, "Any LLM can ask; none can rewrite the rules", 9);
  addRule(slide, 42, 132, 1196);
  labelBox(slide, "LLM clients", "OpenAI models\nAnthropic models\nLocal or future models", {
    left: 42, top: 202, width: 286, height: 220,
  });
  addArrow(slide, 342, 292, 48);
  labelBox(slide, "Thin MCP / tool boundary", "Evaluate a deck\nCompare candidates\nRun a scenario\nExplain a replay", {
    left: 406, top: 178, width: 360, height: 268,
  }, { fill: C.accentPale, lineFill: C.accentStrong, lineWidth: 2, labelSize: 26, bodySize: 20 });
  addArrow(slide, 780, 292, 48);
  labelBox(slide, "Rust engine", "Returns legal actions, simulations, scores, checkpoints, and replay evidence", {
    left: 844, top: 202, width: 394, height: 220,
  });
  addBox(slide, { left: 42, top: 514, width: 558, height: 92 }, { fill: C.accentPale });
  addText(slide, "Allowed", { left: 66, top: 534, width: 120, height: 28 }, { fontSize: 22, bold: true, color: C.accentStrong });
  addText(slide, "propose • request • rank • explain", { left: 184, top: 534, width: 378, height: 30 }, { fontSize: 21 });
  addBox(slide, { left: 638, top: 514, width: 600, height: 92 }, { fill: C.panel });
  addText(slide, "Never", { left: 662, top: 534, width: 110, height: 28 }, { fontSize: 22, bold: true });
  addText(slide, "mutate state • invent actions • define legality", { left: 776, top: 534, width: 426, height: 30 }, { fontSize: 21 });
  addText(slide, "The MCP server is an adapter, not a referee.", { left: 42, top: 634, width: 1196, height: 34 }, {
    fontSize: 21, bold: true, alignment: "center", color: C.muted,
  });
  notes(slide, ["MCP is the intended model-neutral tool interface. The current repository exposes native/CLI boundaries first; the MCP adapter follows the same authority contract."], [
    "C:\\src\\sorcery-tcg\\AGENTS.md",
    "C:\\src\\sorcery-tcg\\crates\\sorcery-engine\\src\\bin\\sorcery-engine.rs",
    "C:\\src\\sorcery-tcg\\crates\\sorcery-engine\\src\\contract.rs",
  ]);
}

// 10 — Milestones, Codex Grid slide-17 hierarchy.
{
  const slide = newSlide();
  addTitle(slide, "The milestones end with one engine and measured speed", 10);
  addRule(slide, 42, 332, 1196, C.ink, 2);
  const x = [42, 452, 862];
  const labels = ["NOW", "CUTOVER", "MATURE LOOP"];
  const headings = ["Rust foundation works", "Finish authority migration", "Optimize decks continuously"];
  const body = [
    "Compact state, checkpoints, simulation, replay, self-play gates, deterministic deck comparison, private catalog, 81/160 direct proofs.",
    "Port remaining supported mechanics, shadow parity, move all consumers to Rust, delete the TypeScript legality engine.",
    "Permissioned deck corpus + prices, cost-aware search, reliable self-play, MCP tools, all-core release profiling.",
  ];
  for (let index = 0; index < x.length; index += 1) {
    addBox(slide, { left: x[index], top: 323, width: 18, height: 18 }, { geometry: "ellipse", fill: index === 0 ? C.accentStrong : C.ink });
    addText(slide, labels[index], { left: x[index], top: 266, width: 170, height: 28 }, { fontSize: 18, bold: true, color: C.muted });
    addText(slide, headings[index], { left: x[index], top: 384, width: 332, height: 54 }, { fontSize: 25, bold: true });
    addText(slide, body[index], { left: x[index], top: 456, width: 332, height: 144 }, { fontSize: 18, color: C.muted });
  }
  addText(slide, "Performance targets: ≥10,000 transitions/s/core for cutover; 25,000–50,000 where mature workloads permit.", {
    left: 42, top: 628, width: 1150, height: 34,
  }, { fontSize: 18, bold: true, color: C.accentStrong });
  notes(slide, ["Current coverage count is exact for the checked-in catalog. Performance numbers are project targets, not measured results from this deck."], [
    "C:\\src\\sorcery-tcg\\data\\rules\\catalog.json",
    "C:\\src\\sorcery-tcg\\docs\\self-play-reliability.md",
    "C:\\src\\sorcery-tcg\\AGENTS.md",
    "User-authored migration target in this task",
  ]);
}

// 11 — Close.
{
  const slide = newSlide();
  addText(slide, "THE DESTINATION", { left: 42, top: 48, width: 400, height: 32 }, {
    fontSize: 20, bold: true, color: C.accentStrong,
  });
  addText(slide, "A trusted simulator first.\nAn optimization laboratory second.\nAn AI interface third.", {
    left: 42, top: 156, width: 1110, height: 248,
  }, { fontSize: 60, bold: true });
  addRule(slide, 42, 456, 1196, C.accentStrong, 5);
  addText(slide, "Compare known decks", { left: 42, top: 504, width: 280, height: 36 }, { fontSize: 23, bold: true });
  addText(slide, "Optimize price / power", { left: 360, top: 504, width: 300, height: 36 }, { fontSize: 23, bold: true });
  addText(slide, "Train deterministic policy", { left: 704, top: 504, width: 330, height: 36 }, { fontSize: 23, bold: true });
  addText(slide, "Explain with any LLM", { left: 42, top: 570, width: 310, height: 36 }, { fontSize: 23, bold: true });
  addText(slide, "One engine • reproducible evidence • human-reviewable coverage", {
    left: 360, top: 570, width: 810, height: 36,
  }, { fontSize: 23, color: C.muted });
  notes(slide, ["This ordering protects correctness: the model layer consumes evidence but never substitutes for the engine."], [
    "C:\\src\\sorcery-tcg\\AGENTS.md",
    "C:\\src\\sorcery-tcg\\docs\\rules-catalog.md",
    "C:\\src\\sorcery-tcg\\docs\\self-play-reliability.md",
  ]);
}

async function writeBlob(filePath, blob) {
  await fs.mkdir(path.dirname(filePath), { recursive: true });
  await fs.writeFile(filePath, new Uint8Array(await blob.arrayBuffer()));
}

await fs.mkdir(renderDir, { recursive: true });
for (const [index, slide] of presentation.slides.items.entries()) {
  const stem = `slide-${String(index + 1).padStart(2, "0")}`;
  await writeBlob(path.join(renderDir, `${stem}.png`), await presentation.export({ slide, format: "png", scale: 2 }));
  const layout = await slide.export({ format: "layout" });
  await fs.writeFile(path.join(renderDir, `${stem}.layout.json`), await layout.text());
}

await writeBlob(previewPath, await presentation.export({ format: "webp", montage: true, scale: 1 }));
const pptx = await PresentationFile.exportPptx(presentation);
await pptx.save(pptxPath);

const inspection = await presentation.inspect({
  kind: "slide,textbox,shape,chart,table,notes",
  maxChars: 40000,
});
await fs.writeFile(path.join(renderDir, "inspection.ndjson"), inspection.ndjson);
