// Fix RST-style artifacts in the virdant skill markdown files.

const ROOT = "/home/vircodificat/projects/virdant-workspace/virdant/agents/skills/virdant";

const PREAMBLE = `The Virdant skill directory (${ROOT}) must be uniformly Markdown. Some files still contain reStructuredText artifacts. Your job: convert ALL RST artifacts to Markdown in EXACTLY the files assigned to you, touching nothing else in the repo.

RULES:
- RST underlined section headers (a title line followed on the next line by a run of ==== or ---- or ~~~~) become ATX headers: a '====' underline becomes '#'+space (or '##' if the file already begins with a '#'-level title — match each file's existing Markdown heading hierarchy so levels stay consistent); a '----' underline becomes the next level down ('##' or '###'); '~' similarly. Delete the underline line itself. NEVER change anything inside fenced code blocks (between \`\`\` lines).
- RST double-backtick inline literals (\`\`foo\`\`) become single-backtick Markdown code spans (\`foo\`), outside code fences only. (Inside fences, leave alone.)
- Watch for other RST leftovers: '.. code-block::', '.. note::', ':ref:', definition lists — convert or drop appropriately, outside fences only.
- Preserve all wording and content; this is a formatting conversion only.
- Keep the style contract: no tabs, no trailing whitespace, lines under 100 chars, 4-space code indent.
- VERIFY when done, for each assigned file: (1) grep -cE '^(-{4,}|={4,}|~{4,})[[:space:]]*$' == 0 outside fences; (2) no double-backtick literals outside fences; (3) heading hierarchy is sane and consistent with sibling files (other files use '# Title' then '## Section'); (4) git diff of your files shows only formatting changes.
- End your output with a 'BUGS:' section (repo bugs found, one line each) — likely none; write 'BUGS: none' if so.`;

const tasks = [
  {
    key: "rst1-headers-files",
    agent: "worker",
    cwd: ROOT,
    task: PREAMBLE + `

ASSIGNED FILES (these two have BOTH underlined headers and double-backtick literals):
- coding-conventions.md
- driver-blocks.md`,
  },
  {
    key: "rst2-heavy-backticks",
    agent: "worker",
    cwd: ROOT,
    task: PREAMBLE + `

ASSIGNED FILES (Markdown headers already; only double-backtick literals and other RST leftovers):
- diagnostics.md
- types-and-patterns.md
- literals-and-builtins.md`,
  },
  {
    key: "rst3-modules-group",
    agent: "worker",
    cwd: ROOT,
    task: PREAMBLE + `

ASSIGNED FILES (Markdown headers already; only double-backtick literals and other RST leftovers):
- modules.md
- sockets.md
- dependson.md
- generics.md`,
  },
  {
    key: "rst4-light-files",
    agent: "worker",
    cwd: ROOT,
    task: PREAMBLE + `

ASSIGNED FILES (small numbers of double-backtick literals; check headers too):
- control-flow.md
- let-bindings.md
- idioms.md
- platforms.md
- tools.md
- tristate.md
- SKILL.md`,
  },
];

const results = await runs.all(tasks);

return results.map((r, i) => ({
  key: tasks[i].key,
  ok: r.ok,
  runId: r.runId,
  outputTail: (r.output || "").slice(-2500),
}));
