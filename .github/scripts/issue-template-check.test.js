// Self-test of issue-template-check.js: `node .github/scripts/issue-template-check.test.js` (exit 1 on a failure).
const assert = require('node:assert/strict');
const { check, comment, MARKER } = require('./issue-template-check.js');

const ids = (body) => check(body).missing.map((q) => q.id);

// 1. A complete issue form body.
const form = `### Who will work on the fix?

I plan to open a pull request with the fix

### al-runner version

al-runner 3.0.0-alpha.14 (1ab46b441)

### Operating system

Ubuntu 24.04

### Command

\`\`\`shell
al-runner test ./app
\`\`\`

### What al-runner did

not run: a refusal

### What you expected

a pass

### Does the test pass on real Business Central?

Yes

### Can you share the AL code?

_No response_`;
assert.deepEqual(ids(form), []);
assert.equal(check(form).fixOwner, 'reporter');

// 2. A form body with empty optional answers and "only reporting".
const sparse = form
  .replace('I plan to open a pull request with the fix', 'I am only reporting it')
  .replace('Ubuntu 24.04', '_No response_')
  .replace('a pass', '_No response_');
assert.deepEqual(ids(sparse), ['os', 'expected']);
assert.equal(check(sparse).fixOwner, 'maintainer');

// 3. The old Markdown template, filled in with bold labels; no fix-owner answer.
const old = `**al-runner version** al-runner 3.0.0-alpha.13

**Operating system** Windows 11 x64

**Command**
\`\`\`
al-runner test App App.Test
\`\`\`

**What al-runner did** failed with an error

**What you expected** a pass

**Does the test pass on real Business Central?** yes`;
assert.deepEqual(ids(old), ['fix-owner']);
assert.equal(check(old).fixOwner, 'unknown');

// 4. A freehand API body with its own headings.
const api = `Build: \`al-runner 3.0.0-alpha.14\`, Linux x64.

## What al-runner does

The test is not run.

## What BC does

Not measured.

## Repro

\`\`\`al
codeunit 50100 X { }
\`\`\``;
assert.deepEqual(ids(api), ['fix-owner', 'command', 'real-bc']);
assert.deepEqual(ids(api.replace('Linux x64', 'somewhere')), ['fix-owner', 'os', 'command', 'real-bc']);

// 5. An empty body asks every question.
assert.equal(ids('').length, 7);

// 6. A choice answer that is not one of the options does not count.
assert.deepEqual(ids(form.replace('\nYes\n', '\nmaybe\n')), ['real-bc']);

// 7. The comment carries the marker and lists exactly the missing questions.
const c = comment(check(old).missing);
assert.ok(c.startsWith(MARKER));
assert.equal((c.match(/^- /gm) || []).length, 1);
assert.ok(comment([]).includes('every question'));

console.log('issue-template-check: all self-tests passed');
