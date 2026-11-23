import assert from 'node:assert/strict';
import test from 'node:test';
import activate from '../src/extension.ts';

test('registers codebase_lookup and returns a miss without echoing questions', async () => {
  const tools = [];
  activate({ registerTool: (tool) => tools.push(tool) });

  assert.equal(tools.length, 1);
  const [tool] = tools;
  assert.equal(tool.name, 'codebase_lookup');
  assert.deepEqual(tool.parameters.required, ['question']);
  assert.equal(tool.parameters.properties.question.type, 'string');

  const context = { sessionManager: { getSessionFile: () => undefined } };
  for (const question of ['Where are requests routed?', 'How do writes reach storage?']) {
    const response = await tool.execute('lookup-test', { question }, undefined, undefined, context);
    assert.deepEqual(JSON.parse(response.content[0].text), {
      status: 'miss',
      message: 'No codebase memory is available. Continue by inspecting the source.',
      findings: [],
    });
    assert.deepEqual(response.details, {});
  }
});
