// Checks an issue body against the questions of .github/ISSUE_TEMPLATE/alpha-feedback.yml.
// Used by .github/workflows/issue-template-check.yml; self-test: `node .github/scripts/issue-template-check.test.js`.
//
// An issue made from the form has one "### <label>" heading per question, and "_No response_" for an empty one.
// An issue made through the API or by hand often has neither, so a question also counts as answered under a bold
// label ("**al-runner version**", the old Markdown template) or a heading of any level that names it.

const MARKER = '<!-- issue-template-check -->';

// Each question: id, the text asked again, the words that name it in a heading or bold label, for a choice the
// accepted answers (a regular expression per answer), and for a few a pattern that answers it anywhere in the body.
const QUESTIONS = [
  {
    id: 'fix-owner',
    ask: 'Who will work on the fix? Please answer "I am only reporting it", "I plan to open a pull request with the fix" or "Not sure yet".',
    names: [/who will (work on|fix)/i, /\bfix owner\b/i],
    choices: { reporter: /pull request|\bPR\b|I (will|plan to) (fix|implement)/i, maintainer: /only report/i, unknown: /not sure/i },
  },
  { id: 'version', ask: 'Which al-runner version did you use (the output of `al-runner --version`)?', names: [/al-?runner version/i, /^version$/i], anywhere: /al-?runner[^\n]{0,20}\b\d+\.\d+\.\d+/i },
  { id: 'os', ask: 'Which operating system did you run it on?', names: [/operating system/i, /^os$/i], anywhere: /\b(windows|linux|macos|ubuntu|debian)\b/i },
  { id: 'command', ask: 'What was the full command line?', names: [/^command/i] },
  { id: 'actual', ask: 'What did al-runner do (passed, failed, not run with its reason, crash or wrong output), with the lines that matter?', names: [/what al-?runner (did|does|do)\b/i, /what (happens|happened)/i, /\bactual\b/i] },
  { id: 'expected', ask: 'What did you expect instead?', names: [/what you expected/i, /expected/i, /what bc does/i] },
  {
    id: 'real-bc',
    ask: 'Does the test pass on real Business Central (yes, no or not tried)?',
    names: [/real business central/i, /pass on (real )?bc/i],
    choices: { yes: /^\s*yes\b/i, no: /^\s*no\b/i, 'not tried': /not tried/i },
  },
];

// Splits a body into [title, text] sections at "#..# Title" headings and at lines that start with "**Title**".
function sections(body) {
  const out = [];
  let title = null;
  let text = [];
  const flush = () => { if (title !== null) out.push([title, text.join('\n').trim()]); };
  for (const line of (body || '').split(/\r?\n/)) {
    const h = line.match(/^\s{0,3}#{1,6}\s+(.+?)\s*#*\s*$/);
    const b = line.match(/^\s*\*\*(.+?)\*\*\s*(.*)$/);
    if (h || b) {
      flush();
      title = (h ? h[1] : b[1]).replace(/[:*]+$/, '').trim();
      text = h ? [] : [b[2]];
    } else {
      text.push(line);
    }
  }
  flush();
  return out;
}

function answered(text) {
  const t = (text || '').replace(/<!--[\s\S]*?-->/g, '').trim();
  return t !== '' && !/^_no response_$/i.test(t);
}

// Returns { missing: [question], fixOwner: 'reporter' | 'maintainer' | 'unknown' }.
function check(body) {
  const secs = sections(body);
  const missing = [];
  let fixOwner = 'unknown';
  for (const q of QUESTIONS) {
    const sec = secs.find(([title]) => q.names.some((n) => n.test(title)));
    const text = sec ? sec[1] : '';
    // A version or operating system named anywhere in a freehand body counts as an answer.
    let ok = answered(text) || (!sec && q.anywhere ? q.anywhere.test(body || '') : false);
    if (ok && q.choices) {
      const hit = Object.entries(q.choices).find(([, re]) => re.test(text));
      ok = Boolean(hit);
      if (hit && q.id === 'fix-owner') fixOwner = hit[0];
    }
    if (!ok) missing.push(q);
  }
  return { missing, fixOwner };
}

function comment(missing) {
  if (missing.length === 0) {
    return `${MARKER}\nThanks, every question of the issue template is answered now.`;
  }
  const list = missing.map((q) => `- ${q.ask}`).join('\n');
  return `${MARKER}\nThanks for the report. Some questions of the issue template are not answered yet:\n\n${list}\n\n` +
    'You can edit the issue or answer in a comment. This check runs again when the issue is edited.';
}

module.exports = { MARKER, QUESTIONS, sections, check, comment };
