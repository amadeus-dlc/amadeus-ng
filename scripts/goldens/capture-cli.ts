#!/usr/bin/env bun
/**
 * CLI 主要遷移とフック代表ケースの実行出力ゴールデンの採取 (FR7.2 / BR2.1 / BR2.4)。
 *
 * upstream ピン `3c3146cf` の配布シェル `dist/claude/` を使い捨てワークスペースに置き、
 * そのピンのツールを **実行して** 観測を採る。各ケースで採るもの:
 *
 * - cli 族   … argv / stdin / stdout / stderr / 終了コード / `aidlc-state.md` の差分 / 監査行の追記分
 * - hook 族  … stdin JSON / stdout / stderr / 終了コード / 監査行の追記分
 *
 * 観測はすべて `normalization.json` の規則 (BR2.2) で正規化してから書く。非対話で
 * 再現できない遷移は `cases-missing.json` に理由付きで記録し、値を捏造しない (W4)。
 *
 * 呼び出しは `scripts/goldens/recapture-cli.sh` 経由 (ピンの取得と sha256 照合は
 * シェル側の責務)。
 */

import { spawnSync } from "node:child_process";
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  realpathSync,
  rmSync,
  statSync,
  utimesSync,
  writeFileSync,
} from "node:fs";
import { hostname, tmpdir } from "node:os";
import { dirname, join } from "node:path";

// --- 正規化 (BR2.2) ---------------------------------------------------------

type Rule = {
  placeholder: string;
  kind: "regex" | "runtime-path" | "runtime-clone";
  pattern: string;
  replacement?: string;
  applies_to: string[];
};

type Normalization = {
  rules: Rule[];
  families: Record<string, { applies: string[] }>;
};

/** 実行時にしか分からない値。規則 `kind` が `runtime-*` のものへ与える。 */
type RuntimeValues = {
  /** 使い捨てワークスペースの絶対パス (raw と realpath の両方)。長い順に置換する。 */
  roots: string[];
  /** 監査シャードの basename と採取ホスト名。 */
  clones: string[];
};

/**
 * 1 つのチャネル (stdout / stderr / state-diff / audit) のテキストを正規化する。
 * 規則は配列順に適用する — 順序は意味を持つ (`<TS>` を先に潰してから `<CLONE>` を当てる)。
 */
function normalize(
  text: string,
  norm: Normalization,
  family: string,
  channel: string,
  runtime: RuntimeValues,
): string {
  const allowed = new Set(norm.families[family]?.applies ?? []);
  let out = text;
  for (const rule of norm.rules) {
    if (!allowed.has(rule.placeholder)) continue;
    if (!rule.applies_to.includes(channel)) continue;
    const replacement = rule.replacement ?? rule.placeholder;
    switch (rule.kind) {
      case "regex":
        out = out.replace(new RegExp(rule.pattern, "g"), replacement);
        break;
      case "runtime-path":
        for (const root of [...runtime.roots].sort((a, b) => b.length - a.length)) {
          out = out.split(root).join(replacement);
        }
        break;
      case "runtime-clone":
        for (const clone of [...runtime.clones].sort((a, b) => b.length - a.length)) {
          if (clone.length === 0) continue;
          out = out.split(clone).join(replacement);
        }
        break;
    }
  }
  return out;
}

// --- 最小の unified diff ----------------------------------------------------

/**
 * 行単位の LCS から unified diff (文脈 3 行) を組み立てる。
 *
 * `aidlc-state.md` は 100 行規模なので O(n*m) の素朴な LCS で十分であり、外部の
 * `diff` コマンドに依存しないぶん採取が再現しやすい。ハンクヘッダは
 * `@@ -<開始>,<行数> +<開始>,<行数> @@` (1 始まり)。
 */
function unifiedDiff(before: string, after: string, context = 3): string {
  if (before === after) return "";
  const a = before.split("\n");
  const b = after.split("\n");
  const n = a.length;
  const m = b.length;

  // lcs[i][j] = a[i..] と b[j..] の最長共通部分列の長さ
  const lcs: number[][] = Array.from({ length: n + 1 }, () => new Array<number>(m + 1).fill(0));
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      lcs[i][j] = a[i] === b[j] ? lcs[i + 1][j + 1] + 1 : Math.max(lcs[i + 1][j], lcs[i][j + 1]);
    }
  }

  type Op = { tag: " " | "-" | "+"; text: string; aIdx: number; bIdx: number };
  const ops: Op[] = [];
  let i = 0;
  let j = 0;
  while (i < n && j < m) {
    if (a[i] === b[j]) {
      ops.push({ tag: " ", text: a[i], aIdx: i, bIdx: j });
      i++;
      j++;
    } else if (lcs[i + 1][j] >= lcs[i][j + 1]) {
      ops.push({ tag: "-", text: a[i], aIdx: i, bIdx: j });
      i++;
    } else {
      ops.push({ tag: "+", text: b[j], aIdx: i, bIdx: j });
      j++;
    }
  }
  for (; i < n; i++) ops.push({ tag: "-", text: a[i], aIdx: i, bIdx: j });
  for (; j < m; j++) ops.push({ tag: "+", text: b[j], aIdx: i, bIdx: j });

  // 変更行の周囲 `context` 行だけを残してハンクに束ねる。
  const keep = new Array<boolean>(ops.length).fill(false);
  for (let k = 0; k < ops.length; k++) {
    if (ops[k].tag === " ") continue;
    for (let d = Math.max(0, k - context); d <= Math.min(ops.length - 1, k + context); d++) {
      keep[d] = true;
    }
  }

  const lines: string[] = ["--- aidlc-state.md (before)", "+++ aidlc-state.md (after)"];
  let k = 0;
  while (k < ops.length) {
    if (!keep[k]) {
      k++;
      continue;
    }
    const start = k;
    while (k < ops.length && keep[k]) k++;
    const hunk = ops.slice(start, k);
    const aStart = hunk[0].aIdx + 1;
    const bStart = hunk[0].bIdx + 1;
    const aCount = hunk.filter((o) => o.tag !== "+").length;
    const bCount = hunk.filter((o) => o.tag !== "-").length;
    lines.push(`@@ -${aStart},${aCount} +${bStart},${bCount} @@`);
    for (const op of hunk) lines.push(`${op.tag}${op.text}`);
  }
  return `${lines.join("\n")}\n`;
}

// --- 使い捨てワークスペース --------------------------------------------------

const NON_INTERACTIVE_ENV = {
  // 人間の在席確認。非対話の採取では人間のターンが存在しない。
  AIDLC_SKIP_HUMAN_PRESENCE_GUARD: "1",
  // 合議 (ensemble) の寄稿ファイル検査。寄稿は実ファイルを置いて満たすが、
  // practices-promote 側の検査は寄稿の内容審査まで行うため無効化する。
  AIDLC_DISABLE_ENSEMBLE_EVIDENCE: "1",
  // 質問フロー (<slug>-questions.md) と人間承認の受領証。非対話では作れない。
  AIDLC_SKIP_SUMMARY_CONFIRMATION_GUARD: "1",
  // 成果物の存在・鮮度検査。採取の主題は遷移の出力であって成果物ではない。
  AIDLC_SKIP_ARTIFACT_GUARD: "1",
  // 利用量トラッキング。採取ごとに変わる値をゴールデンへ混ぜない。
  AIDLC_DISABLE_USAGE_TRACKING: "1",
} as const;

type Workspace = {
  dir: string;
  roots: string[];
};

function makeWorkspace(distDir: string, label: string): Workspace {
  const dir = mkdtempSync(join(tmpdir(), `aidlc-golden-${label}-`));
  cpSync(join(distDir, ".claude"), join(dir, ".claude"), { recursive: true });
  cpSync(join(distDir, "aidlc"), join(dir, "aidlc"), { recursive: true });
  const roots = [dir];
  try {
    const real = realpathSync(dir);
    if (real !== dir) roots.push(real);
  } catch {
    // realpath が取れなくても raw パスの正規化だけで進める。
  }
  return { dir, roots };
}

const INTENTS_DIR = "aidlc/spaces/default/intents";

/**
 * 使い捨てワークスペースで `intent-create --label` に渡すラベル。記録ディレクトリ名は
 * `<YYMMDD>-<ラベル>` になり、日付部分は `normalization.json` の `\d{6}-golden` 規則が
 * `<TS>` へ潰す。ラベルを変えるなら規則も一緒に変える。
 */
const FIXTURE_INTENT_LABEL = "golden";

function recordDir(ws: Workspace): string | null {
  const base = join(ws.dir, INTENTS_DIR);
  if (!existsSync(base)) return null;
  const entries = readdirSync(base, { withFileTypes: true })
    .filter((e) => e.isDirectory())
    .map((e) => e.name);
  return entries.length === 1 ? join(base, entries[0]) : null;
}

/** 記録ディレクトリのワークスペース相対パス (`aidlc/spaces/<space>/intents/<YYMMDD>-<label>`)。 */
function recordRel(ws: Workspace): string {
  const abs = recordDir(ws);
  if (abs === null) throw new Error("記録ディレクトリがまだ無い (intent-create より前に呼ばれた)");
  return abs.slice(ws.dir.length + 1);
}

function stateText(ws: Workspace): string {
  const rec = recordDir(ws);
  if (!rec) return "";
  const p = join(rec, "aidlc-state.md");
  return existsSync(p) ? readFileSync(p, "utf-8") : "";
}

function auditText(ws: Workspace): string {
  const rec = recordDir(ws);
  if (!rec) return "";
  const dir = join(rec, "audit");
  if (!existsSync(dir)) return "";
  return readdirSync(dir)
    .filter((f) => f.endsWith(".md"))
    .sort()
    .map((f) => readFileSync(join(dir, f), "utf-8"))
    .join("");
}

function cloneNames(ws: Workspace): string[] {
  const names = new Set<string>([hostname()]);
  const rec = recordDir(ws);
  if (rec) {
    const dir = join(rec, "audit");
    if (existsSync(dir)) {
      for (const f of readdirSync(dir)) {
        if (f.endsWith(".md")) names.add(f.slice(0, -3));
      }
    }
  }
  return [...names].filter((n) => n.length > 0);
}

// --- ケースの記述と実行 ------------------------------------------------------

/** 記録するケース 1 件。`capture` が false のステップは前提を整えるだけで記録しない。 */
type CliStep = {
  /** 記録しないなら null。記録するなら `<verb>/<case>`。 */
  id: string | null;
  description: string;
  tool: string;
  /** `--project-dir` は実行時に足す。ゴールデンの argv には `<ROOT>` として書く。 */
  args: string[];
  /** 実行時にしか決まらない引数 (記録ディレクトリ名など) を持つステップはこちらで組む。 */
  argsFn?: () => string[];
  /** ゴールデンの argv に書く形 (トークン等の非決定値をプレースホルダに置いたもの)。 */
  argvForGolden?: string[];
  stdin?: string;
  /** 実行前に整える前提 (成果物の作成など)。 */
  setup?: () => void;
};

type Missing = { id: string; reason: string; evidence: string; follow_up: string };

function main(): void {
  const [distDir, outDir, metaPath] = process.argv.slice(2);
  if (!distDir || !outDir || !metaPath) {
    throw new Error(
      "Usage: bun capture-cli.ts <dist-claude-dir> <golden-out-dir> <meta.json>",
    );
  }

  const meta = JSON.parse(readFileSync(metaPath, "utf-8")) as Record<string, unknown>;
  const norm = JSON.parse(
    readFileSync(join(outDir, "normalization.json"), "utf-8"),
  ) as Normalization;
  const capturedAt = new Date().toISOString().replace(/\.\d{3}Z$/, "Z");

  const missingCli: Missing[] = [];
  const missingHooks: Missing[] = [];

  const cliCount = captureCli(distDir, outDir, norm, meta, capturedAt, missingCli);
  const hookCount = captureHooks(distDir, outDir, norm, meta, capturedAt, missingHooks);

  scanForLeaks(join(outDir, "cli"));
  scanForLeaks(join(outDir, "hooks"));

  process.stdout.write(
    `    cli: ${cliCount} ケース (欠落 ${missingCli.length}) / hooks: ${hookCount} ケース (欠落 ${missingHooks.length})\n`,
  );
}

// --- cli 族 -----------------------------------------------------------------

function captureCli(
  distDir: string,
  outDir: string,
  norm: Normalization,
  meta: Record<string, unknown>,
  capturedAt: string,
  missing: Missing[],
): number {
  const ws = makeWorkspace(distDir, "cli");
  const family = "cli";
  const familyDir = join(outDir, family);
  rmSync(familyDir, { recursive: true, force: true });

  // 記録ディレクトリ名は `<YYMMDD>-<label>` で採取日に依存するため、intent-create の
  // あとに実測から解決する (ハードコードすると別の日に再採取できない)。
  const pd = (): string => `${recordRel(ws)}/inception/practices-discovery`;
  const write = (rel: string, body: string): void => {
    const p = join(ws.dir, rel);
    mkdirSync(dirname(p), { recursive: true });
    writeFileSync(p, body);
  };

  // 直前の `next` が返した継続トークン。`continue` ケースが使う。
  let steeringToken = "";

  const steps: CliStep[] = [
    {
      id: "next/no-active-intent",
      description: "作業がまだ 1 件も無いワークスペースで next を呼ぶと、開始コマンドを名指す print directive が返る",
      tool: "aidlc-orchestrate.ts",
      args: ["next", "--scope", "classic"],
    },
    {
      id: "intent-create/classic-scope",
      description: "classic スコープで最初の intent を作り、状態ファイル・監査シャード・記録ディレクトリを起こす",
      tool: "aidlc-utility.ts",
      args: [
        "intent-create",
        "--scope",
        "classic",
        "--label",
        FIXTURE_INTENT_LABEL,
        "--arguments",
        "Build a small ordering service",
      ],
    },
    {
      id: "next/start",
      description: "intent 作成後の最初の next。最初の実行ステージの規則束を load-steering で配る",
      tool: "aidlc-orchestrate.ts",
      args: ["next"],
    },
    {
      id: "continue/load-steering",
      description: "load-steering の継続トークンを渡すと run-stage directive が返る",
      tool: "aidlc-orchestrate.ts",
      args: [], // steeringToken が決まってから埋める
      argvForGolden: ["continue", "<SESSION>"],
    },
    {
      id: "continue/invalid-token",
      description: "壊れた継続トークンは error directive で拒否される (終了コードは 0 のまま)",
      tool: "aidlc-orchestrate.ts",
      args: ["continue", "not-a-continuation-token"],
    },
    {
      id: "report/awaiting-approval",
      description: "ステージの成果物を出し終えて承認待ちゲートを開く",
      tool: "aidlc-orchestrate.ts",
      args: ["report", "--result", "awaiting-approval"],
      setup: () => {
        const dir = pd();
        write(`${dir}/team-practices.md`, "# team-practices\n\n採取用の最小成果物。\n");
        write(`${dir}/discovered-rules.md`, "# discovered-rules\n\n採取用の最小成果物。\n");
        write(`${dir}/evidence.md`, "# evidence\n\n採取用の最小成果物。\n");
        for (const agent of [
          "aidlc-quality-agent",
          "aidlc-developer-agent",
          "aidlc-devsecops-agent",
        ]) {
          write(
            `${dir}/contributions/${agent}.md`,
            `**Collaborator:** ${agent}\n\n## Positions\n\nAGREE: 採取用の最小寄稿。\n`,
          );
        }
      },
    },
    {
      id: "report/rejected",
      description: "開いているゲートを人間が差し戻す",
      tool: "aidlc-orchestrate.ts",
      args: ["report", "--result", "rejected", "--reason", "Sharpen the testing posture."],
    },
    {
      id: "report/revised",
      description: "差し戻しを受けて直したことを報告し、ゲートを開き直す",
      tool: "aidlc-orchestrate.ts",
      args: [
        "report",
        "--result",
        "revised",
        "--user-input",
        "Tightened the testing posture.",
      ],
    },
    {
      id: "report/awaiting-approval-repeat",
      description: "すでに承認待ちのステージに再度 awaiting-approval を報告しても二重に開かない (冪等)",
      tool: "aidlc-orchestrate.ts",
      args: ["report", "--result", "awaiting-approval"],
    },
    {
      id: "practices-promote/affirm",
      description: "承認の前提。affirm したプラクティスを team.md / project.md へ昇格し PRACTICES_AFFIRMED を記録する",
      tool: "aidlc-state.ts",
      args: [],
      argsFn: () => [
        "practices-promote",
        "--team-practices",
        `${pd()}/team-practices.md`,
        "--discovered-rules",
        `${pd()}/discovered-rules.md`,
      ],
      argvForGolden: [
        "practices-promote",
        "--team-practices",
        "aidlc/spaces/default/intents/<TS>-golden/inception/practices-discovery/team-practices.md",
        "--discovered-rules",
        "aidlc/spaces/default/intents/<TS>-golden/inception/practices-discovery/discovered-rules.md",
      ],
    },
    {
      id: "report/approved",
      description: "ゲートを承認して次ステージへ進める",
      tool: "aidlc-orchestrate.ts",
      args: ["report", "--result", "approved", "--user-input", "A"],
    },
    {
      id: "next/after-approval",
      description: "承認直後の next。次のステージの規則束を配る",
      tool: "aidlc-orchestrate.ts",
      args: ["next"],
    },
    {
      id: "jump/execute-forward-to-conditional",
      description: "条件付きステージへ前方ジャンプし、間のステージを [S] で飛ばす",
      tool: "aidlc-jump.ts",
      args: ["execute", "--target", "user-stories", "--direction", "forward"],
    },
    {
      id: "skip/skipped",
      description: "条件付きの現ステージを skipped として確定し、前へ流す",
      tool: "aidlc-orchestrate.ts",
      args: [
        "report",
        "--result",
        "skipped",
        "--stage",
        "user-stories",
        "--reason",
        "No UI surface in this workflow.",
      ],
    },
    {
      id: "jump/resolve-forward",
      description: "前方ジャンプの可否と影響範囲を問い合わせる (状態は変えない)",
      tool: "aidlc-jump.ts",
      args: ["resolve", "--stage", "domain-design"],
    },
    {
      id: "jump/execute-forward",
      description: "問い合わせた前方ジャンプを実行する",
      tool: "aidlc-jump.ts",
      args: ["execute", "--target", "domain-design", "--direction", "forward"],
    },
    {
      id: "next/stage-jump-print",
      description: "next --stage は自分でジャンプせず、実行すべき jump コマンドを print directive で名指す",
      tool: "aidlc-orchestrate.ts",
      args: ["next", "--stage", "contract-design"],
    },
    {
      id: "recompose/skip-one",
      description: "実行中の計画から保留ステージを 1 本外す",
      tool: "aidlc-utility.ts",
      args: ["recompose", "--skip", "incident-response"],
    },
    {
      id: "recompose/rejected-starved-input",
      description: "下流の必須入力を枯らす recompose は strict validator が拒否する",
      tool: "aidlc-utility.ts",
      args: ["recompose", "--skip", "observability-setup"],
    },
    {
      id: "park/park",
      description: "作業を保存して一時停止する",
      tool: "aidlc-orchestrate.ts",
      args: ["park"],
    },
    {
      id: "unpark/unpark",
      description: "一時停止を解除して再開できる状態に戻す",
      tool: "aidlc-state.ts",
      args: ["unpark"],
    },
    {
      id: "set-autonomy/state-field-absent",
      description:
        "ピン 3c3146cf の intent-create が起こす状態ファイルには Construction Autonomy Mode 行が無いため、set-autonomy は終了コード 1 で拒否される",
      tool: "aidlc-bolt.ts",
      args: ["set-autonomy", "--mode", "gated"],
    },
  ];

  let captured = 0;
  for (const step of steps) {
    step.setup?.();

    let args = step.argsFn ? step.argsFn() : step.args;
    if (step.id === "continue/load-steering") {
      if (steeringToken.length === 0) {
        missing.push({
          id: "continue/load-steering",
          reason: "直前の next が継続トークンを返さなかったため、continue の正常系を採取できなかった",
          evidence: "next/start の stdout に continue_token フィールドが無い",
          follow_up: "U6 (next / continue ユースケース) で採り直す",
        });
        continue;
      }
      args = ["continue", steeringToken];
    }

    const before = { state: stateText(ws), audit: auditText(ws) };
    const result = spawnSync(
      "bun",
      [join(ws.dir, ".claude/tools", step.tool), ...args, "--project-dir", ws.dir],
      {
        cwd: ws.dir,
        input: step.stdin ?? "",
        encoding: "utf-8",
        env: {
          ...process.env,
          CLAUDE_PROJECT_DIR: ws.dir,
          AIDLC_PROJECT_DIR: ws.dir,
          ...NON_INTERACTIVE_ENV,
        },
      },
    );
    const stdout = result.stdout ?? "";
    const stderr = result.stderr ?? "";
    const after = { state: stateText(ws), audit: auditText(ws) };

    // 次の continue ケースのためにトークンを拾う。
    try {
      const parsed = JSON.parse(stdout) as { continue_token?: unknown };
      if (typeof parsed.continue_token === "string") steeringToken = parsed.continue_token;
    } catch {
      // JSON でない stdout はトークンを持たない。
    }

    if (step.id === null) continue;

    const runtime: RuntimeValues = { roots: ws.roots, clones: cloneNames(ws) };
    const caseDir = join(familyDir, step.id);
    mkdirSync(caseDir, { recursive: true });

    const argvGolden = ["bun", `.claude/tools/${step.tool}`, ...(step.argvForGolden ?? args), "--project-dir", "<ROOT>"];
    writeFileSync(join(caseDir, "argv"), `${JSON.stringify(argvGolden, null, 2)}\n`);
    writeFileSync(join(caseDir, "stdin"), step.stdin ?? "");
    writeFileSync(join(caseDir, "exit"), `${result.status ?? -1}\n`);

    const stdoutNorm = normalize(stdout, norm, family, "stdout", runtime);
    const isJson = ((): boolean => {
      try {
        JSON.parse(stdout);
        return stdout.trim().length > 0;
      } catch {
        return false;
      }
    })();
    writeFileSync(join(caseDir, isJson ? "stdout.json" : "stdout.txt"), stdoutNorm);
    writeFileSync(
      join(caseDir, "stderr"),
      normalize(stderr, norm, family, "stderr", runtime),
    );

    const diff = unifiedDiff(
      normalize(before.state, norm, family, "state-diff", runtime),
      normalize(after.state, norm, family, "state-diff", runtime),
    );
    writeFileSync(join(caseDir, "state.diff"), diff);

    const auditDelta = after.audit.slice(before.audit.length);
    writeFileSync(
      join(caseDir, "audit.md"),
      normalize(auditDelta, norm, family, "audit", runtime),
    );

    writeFileSync(
      join(caseDir, "case.json"),
      `${JSON.stringify(
        {
          id: `${family}/${step.id}`,
          family,
          verb: step.id.split("/")[0],
          case: step.id.split("/")[1],
          description: step.description,
          stdout_kind: isJson ? "json" : "text",
          provenance: {
            commit: meta.upstream_commit,
            captured_at: capturedAt,
            command: meta.command,
          },
        },
        null,
        2,
      )}\n`,
    );
    captured++;
  }

  missing.push({
    id: "set-autonomy/gated",
    reason:
      "set-autonomy の正常系はピン 3c3146cf では非対話でも対話でも到達できない — intent-create が書く状態ファイルのテンプレートに `- **Construction Autonomy Mode**:` 行が無く、set-autonomy は setFieldStrict で行が無いことを検出して終了コード 1 で止まる",
    evidence:
      "aidlc-utility.ts の状態ファイルテンプレートに当該行が無い一方、knowledge/aidlc-shared/state-template.md は当該行を規定している。実測は cli/set-autonomy/state-field-absent",
    follow_up:
      "U7 (CLI ディスパッチャ) でこの upstream の欠落を逸脱台帳と突き合わせ、正常系が必要なら追加採取する",
  });
  missing.push({
    id: "continue/multi-part",
    reason:
      "規則束が 28 KiB 上限を超えたときの分割配送 (parts > 1) を非対話で再現できなかった — 配布シェルの既定メモリでは規則束が 1 パートに収まり part=1/parts=1 にしかならない",
    evidence: "next/start の stdout が parts=1。分割には 28 KiB を超える memory/*.md を持つワークスペースが要る",
    follow_up: "U6 (next / continue ユースケース) で分割の合成入力を用意して採取する",
  });

  mkdirSync(familyDir, { recursive: true });
  writeFileSync(
    join(familyDir, "cases-missing.json"),
    `${JSON.stringify({ family, upstream_commit: meta.upstream_commit, missing }, null, 2)}\n`,
  );
  writeFileSync(
    join(familyDir, "provenance.json"),
    `${JSON.stringify(
      {
        family,
        upstream_repo: meta.upstream_repo,
        upstream_commit: meta.upstream_commit,
        upstream_version: meta.upstream_version,
        source_path: meta.source_path,
        fetch_method: meta.fetch_method,
        tree_manifest_sha256: meta.tree_manifest_sha256,
        tree_file_count: meta.tree_file_count,
        captured_at: capturedAt,
        command: meta.command,
        bun_version: meta.bun_version,
        non_interactive_env: NON_INTERACTIVE_ENV,
        fixture_intent_label: FIXTURE_INTENT_LABEL,
        case_count: captured,
        missing_case_count: missing.length,
      },
      null,
      2,
    )}\n`,
  );
  rmSync(ws.dir, { recursive: true, force: true });
  return captured;
}

// --- hook 族 ----------------------------------------------------------------

/** フック 1 件の代表ケース。`stdinForGolden` は非決定値をプレースホルダへ置いた記録用。 */
type HookStep = {
  id: string;
  description: string;
  /** 契約上のフック名 (C2) → upstream の実装ファイル名。 */
  hookFile: string;
  /** true なら intent を持たないワークスペースで実行する。 */
  noWorkflow?: boolean;
  stdin: (ws: Workspace) => string;
  setup?: (ws: Workspace) => void;
};

/** C2 のフック 4 本と upstream 実装ファイルの写像。 */
const HOOK_FILES: Record<string, string> = {
  "stop-forwarding-loop": "aidlc-continue-workflow.ts",
  "record-human-turn": "aidlc-record-human-turn.ts",
  "state-transition-guard": "aidlc-state-transition-guard.ts",
  "write-audit-log": "aidlc-write-audit-log.ts",
};

const SESSION_ID = "11111111-2222-4333-8444-555555555555";

function captureHooks(
  distDir: string,
  outDir: string,
  norm: Normalization,
  meta: Record<string, unknown>,
  capturedAt: string,
  missing: Missing[],
): number {
  const family = "hooks";
  const familyDir = join(outDir, family);
  rmSync(familyDir, { recursive: true, force: true });

  const active = makeWorkspace(distDir, "hooks-active");
  const bare = makeWorkspace(distDir, "hooks-bare");

  // active 側だけ intent を起こす。
  spawnSync(
    "bun",
    [
      join(active.dir, ".claude/tools/aidlc-utility.ts"),
      "intent-create",
      "--scope",
      "classic",
      "--label",
      FIXTURE_INTENT_LABEL,
      "--arguments",
      "Build a small ordering service",
      "--project-dir",
      active.dir,
    ],
    {
      cwd: active.dir,
      encoding: "utf-8",
      env: { ...process.env, CLAUDE_PROJECT_DIR: active.dir, AIDLC_PROJECT_DIR: active.dir, ...NON_INTERACTIVE_ENV },
    },
  );

  const artifact = (ws: Workspace, name: string): string =>
    join(ws.dir, recordRel(ws), "inception/practices-discovery", name);

  const steps: HookStep[] = [
    {
      id: "stop-forwarding-loop/block-pending-directive",
      description: "保留中の directive があるターン終了は decision:block で差し止め、続きの指示を reason に載せる",
      hookFile: HOOK_FILES["stop-forwarding-loop"],
      stdin: () =>
        `${JSON.stringify({ hook_event_name: "Stop", stop_hook_active: false, session_id: SESSION_ID, transcript_path: "/dev/null" })}\n`,
    },
    {
      id: "stop-forwarding-loop/reentrant-ignored",
      description: "自分が差し止めた結果の再入 (stop_hook_active) は無視して素通しする",
      hookFile: HOOK_FILES["stop-forwarding-loop"],
      stdin: () =>
        `${JSON.stringify({ hook_event_name: "Stop", stop_hook_active: true, session_id: SESSION_ID, transcript_path: "/dev/null" })}\n`,
    },
    {
      id: "stop-forwarding-loop/no-workflow-ignored",
      description: "作業が 1 件も無いワークスペースでは差し止めるものが無いので素通しする",
      hookFile: HOOK_FILES["stop-forwarding-loop"],
      noWorkflow: true,
      stdin: () =>
        `${JSON.stringify({ hook_event_name: "Stop", stop_hook_active: false, session_id: SESSION_ID, transcript_path: "/dev/null" })}\n`,
    },
    {
      id: "record-human-turn/active-workflow",
      description: "人間の発話ターンを HUMAN_TURN 監査行として記録する (本文は読まない)",
      hookFile: HOOK_FILES["record-human-turn"],
      stdin: () =>
        `${JSON.stringify({ hook_event_name: "UserPromptSubmit", prompt: "Approve the practices." })}\n`,
    },
    {
      id: "record-human-turn/no-workflow-ignored",
      description: "作業が 1 件も無いワークスペースでは監査シャードを作らずに素通しする",
      hookFile: HOOK_FILES["record-human-turn"],
      noWorkflow: true,
      stdin: () => `${JSON.stringify({ hook_event_name: "UserPromptSubmit", prompt: "hello" })}\n`,
    },
    {
      id: "state-transition-guard/deny-direct-state-transition",
      description: "aidlc-state.ts のライフサイクル動詞を直接叩く Bash は終了コード 2 で拒否する",
      hookFile: HOOK_FILES["state-transition-guard"],
      stdin: () =>
        `${JSON.stringify({
          hook_event_name: "PreToolUse",
          tool_name: "Bash",
          tool_input: { command: "bun .claude/tools/aidlc-state.ts approve practices-discovery" },
        })}\n`,
    },
    {
      id: "state-transition-guard/deny-delegated-lifecycle",
      description: "委任されたエージェントによる進行・ルーティング系コマンドは終了コード 2 で拒否する",
      hookFile: HOOK_FILES["state-transition-guard"],
      stdin: () =>
        `${JSON.stringify({
          hook_event_name: "PreToolUse",
          tool_name: "Bash",
          agent_type: "aidlc-developer-agent",
          tool_input: { command: "bun .claude/tools/aidlc-orchestrate.ts next" },
        })}\n`,
    },
    {
      id: "state-transition-guard/allow-read-only-query",
      description: "状態の読み取り専用照会は許可する",
      hookFile: HOOK_FILES["state-transition-guard"],
      stdin: () =>
        `${JSON.stringify({
          hook_event_name: "PreToolUse",
          tool_name: "Bash",
          tool_input: { command: "bun .claude/tools/aidlc-state.ts get Scope" },
        })}\n`,
    },
    {
      id: "state-transition-guard/ignore-non-bash-tool",
      description: "Bash 以外のツール呼び出しは判定対象外として無視する",
      hookFile: HOOK_FILES["state-transition-guard"],
      stdin: () =>
        `${JSON.stringify({
          hook_event_name: "PreToolUse",
          tool_name: "Read",
          tool_input: { file_path: "README.md" },
        })}\n`,
    },
    {
      id: "write-audit-log/artifact-created",
      description: "記録ディレクトリ配下への新規 Write は ARTIFACT_CREATED を残す",
      hookFile: HOOK_FILES["write-audit-log"],
      setup: (ws) => {
        const p = artifact(ws, "team-practices.md");
        mkdirSync(dirname(p), { recursive: true });
        writeFileSync(p, "# team-practices\n");
      },
      stdin: (ws) =>
        `${JSON.stringify({
          hook_event_name: "PostToolUse",
          tool_name: "Write",
          tool_input: { file_path: artifact(ws, "team-practices.md") },
        })}\n`,
    },
    {
      id: "write-audit-log/artifact-updated-by-edit",
      description: "Edit は必ず ARTIFACT_UPDATED (Edit は既存ファイルにしか当たらないため)",
      hookFile: HOOK_FILES["write-audit-log"],
      stdin: (ws) =>
        `${JSON.stringify({
          hook_event_name: "PostToolUse",
          tool_name: "Edit",
          tool_input: { file_path: artifact(ws, "team-practices.md") },
        })}\n`,
    },
    {
      id: "write-audit-log/artifact-updated-by-overwrite",
      description:
        "Write でも mtime が birthtime から離れていれば上書きとみなし ARTIFACT_UPDATED を残す (採取では mtime を 60 秒戻して上書きを模す)",
      hookFile: HOOK_FILES["write-audit-log"],
      setup: (ws) => {
        const p = artifact(ws, "discovered-rules.md");
        mkdirSync(dirname(p), { recursive: true });
        writeFileSync(p, "# discovered-rules\n");
        const st = statSync(p);
        const older = new Date(st.birthtime.getTime() - 60_000);
        utimesSync(p, older, older);
      },
      stdin: (ws) =>
        `${JSON.stringify({
          hook_event_name: "PostToolUse",
          tool_name: "Write",
          tool_input: { file_path: artifact(ws, "discovered-rules.md") },
        })}\n`,
    },
    {
      id: "write-audit-log/ignore-outside-record",
      description: "記録ディレクトリの外への書込は監査に残さない",
      hookFile: HOOK_FILES["write-audit-log"],
      stdin: (ws) =>
        `${JSON.stringify({
          hook_event_name: "PostToolUse",
          tool_name: "Write",
          tool_input: { file_path: join(ws.dir, "README.md") },
        })}\n`,
    },
    {
      id: "write-audit-log/trusts-the-settings-matcher",
      description:
        "フック自身はツール名で絞らない — Write|Edit 以外の tool_name でも記録ディレクトリ配下なら行を残す (絞り込みは settings.json の matcher の責務)",
      hookFile: HOOK_FILES["write-audit-log"],
      setup: (ws) => {
        const p = artifact(ws, "evidence.md");
        mkdirSync(dirname(p), { recursive: true });
        writeFileSync(p, "# evidence\n");
      },
      stdin: (ws) =>
        `${JSON.stringify({
          hook_event_name: "PostToolUse",
          tool_name: "Read",
          tool_input: { file_path: artifact(ws, "evidence.md") },
        })}\n`,
    },
  ];

  let captured = 0;
  for (const step of steps) {
    const ws = step.noWorkflow ? bare : active;
    step.setup?.(ws);

    const stdin = step.stdin(ws);
    const before = auditText(ws);
    const result = spawnSync("bun", [join(ws.dir, ".claude/hooks", step.hookFile)], {
      cwd: ws.dir,
      input: stdin,
      encoding: "utf-8",
      env: {
        ...process.env,
        CLAUDE_PROJECT_DIR: ws.dir,
        AIDLC_PROJECT_DIR: ws.dir,
        ...NON_INTERACTIVE_ENV,
      },
    });
    const after = auditText(ws);

    const runtime: RuntimeValues = { roots: ws.roots, clones: cloneNames(ws) };
    const caseDir = join(familyDir, step.id);
    mkdirSync(caseDir, { recursive: true });

    writeFileSync(
      join(caseDir, "stdin.json"),
      normalize(stdin, norm, family, "stdout", runtime),
    );
    writeFileSync(join(caseDir, "exit"), `${result.status ?? -1}\n`);
    writeFileSync(
      join(caseDir, "stderr"),
      normalize(result.stderr ?? "", norm, family, "stderr", runtime),
    );
    writeFileSync(
      join(caseDir, "stdout"),
      normalize(result.stdout ?? "", norm, family, "stdout", runtime),
    );
    writeFileSync(
      join(caseDir, "audit.md"),
      normalize(after.slice(before.length), norm, family, "audit", runtime),
    );
    writeFileSync(
      join(caseDir, "case.json"),
      `${JSON.stringify(
        {
          id: `${family}/${step.id}`,
          family,
          hook: step.id.split("/")[0],
          case: step.id.split("/")[1],
          upstream_hook_file: `.claude/hooks/${step.hookFile}`,
          description: step.description,
          workspace: step.noWorkflow ? "no-active-intent" : "active-workflow",
          provenance: {
            commit: meta.upstream_commit,
            captured_at: capturedAt,
            command: meta.command,
          },
        },
        null,
        2,
      )}\n`,
    );
    captured++;
  }

  missing.push({
    id: "stop-forwarding-loop/transcript-carve-out",
    reason:
      "会話ターンの切り出し (transcript を読んで会話だけのターンを差し止めない判定) は本物のトランスクリプト JSONL を要するため非対話で再現できなかった",
    evidence: "採取では transcript_path に /dev/null を渡している",
    follow_up: "U7 (フックのサブコマンド) でトランスクリプトの合成入力を用意して採取する",
  });

  mkdirSync(familyDir, { recursive: true });
  writeFileSync(
    join(familyDir, "cases-missing.json"),
    `${JSON.stringify({ family, upstream_commit: meta.upstream_commit, missing }, null, 2)}\n`,
  );
  writeFileSync(
    join(familyDir, "provenance.json"),
    `${JSON.stringify(
      {
        family,
        upstream_repo: meta.upstream_repo,
        upstream_commit: meta.upstream_commit,
        upstream_version: meta.upstream_version,
        source_path: meta.source_path,
        fetch_method: meta.fetch_method,
        tree_manifest_sha256: meta.tree_manifest_sha256,
        tree_file_count: meta.tree_file_count,
        hook_files: HOOK_FILES,
        captured_at: capturedAt,
        command: meta.command,
        bun_version: meta.bun_version,
        non_interactive_env: NON_INTERACTIVE_ENV,
        fixture_intent_label: FIXTURE_INTENT_LABEL,
        case_count: captured,
        missing_case_count: missing.length,
      },
      null,
      2,
    )}\n`,
  );
  rmSync(active.dir, { recursive: true, force: true });
  rmSync(bare.dir, { recursive: true, force: true });
  return captured;
}

// --- 環境固有値の漏れ検査 (NFR4.4) ------------------------------------------

/**
 * 書き終えたコーパスに採取環境の値が残っていないか確かめる。1 件でも見つかったら
 * 停止する — 正規化漏れをコミットさせないための機械的な歯止め。
 */
function scanForLeaks(dir: string): void {
  const host = hostname();
  const user = process.env.USER ?? "";
  const needles: { label: string; value: string }[] = [
    { label: "ホスト名", value: host },
    { label: "ホスト名 (先頭ラベル)", value: host.split(".")[0] },
    { label: "ユーザ名", value: user },
    { label: "ホームディレクトリ", value: process.env.HOME ?? "" },
    { label: "一時ディレクトリ", value: tmpdir() },
  ].filter((n) => n.value.length >= 3);

  const walk = (d: string): string[] => {
    const found: string[] = [];
    for (const entry of readdirSync(d, { withFileTypes: true })) {
      const p = join(d, entry.name);
      if (entry.isDirectory()) {
        found.push(...walk(p));
        continue;
      }
      const text = readFileSync(p, "utf-8");
      for (const needle of needles) {
        if (text.includes(needle.value)) found.push(`${p}: ${needle.label} が残っている`);
      }
    }
    return found;
  };

  if (!existsSync(dir)) return;
  const leaks = walk(dir);
  if (leaks.length > 0) {
    throw new Error(
      `NFR4.4: ゴールデンに環境固有値が残っています (正規化規則を足してください)\n  ${leaks.join("\n  ")}`,
    );
  }
}

main();
