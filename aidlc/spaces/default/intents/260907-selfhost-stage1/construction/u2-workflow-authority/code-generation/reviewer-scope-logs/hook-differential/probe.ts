import { auditFilePath, hooksHealthDir, reviewerDispatchPath, docsRoot } from "../upstream/dist/claude/.claude/tools/aidlc-lib.ts";
const p = process.env.AIDLC_PROJECT_DIR!;
console.log(JSON.stringify({ docsRoot: docsRoot(p), audit: auditFilePath(p), health: hooksHealthDir(p), dispatch: reviewerDispatchPath(p) }));
