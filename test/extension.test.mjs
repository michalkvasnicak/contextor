import assert from 'node:assert/strict';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import activate from '../src/extension.ts';

test('registers codebase_lookup and returns a miss without echoing questions', async (t) => {
  const agentDir = await mkdtemp(join(tmpdir(), 'pi-memory-test-'));
  const previousAgentDir = process.env.PI_CODING_AGENT_DIR;
  process.env.PI_CODING_AGENT_DIR = agentDir;
  t.after(async () => {
    if (previousAgentDir === undefined) delete process.env.PI_CODING_AGENT_DIR;
    else process.env.PI_CODING_AGENT_DIR = previousAgentDir;
    await rm(agentDir, { recursive: true, force: true });
  });

  const tools = [];
  activate({ registerTool: (tool) => tools.push(tool), registerCommand() {}, on() {} });

  assert.equal(tools.length, 1);
  const [tool] = tools;
  assert.equal(tool.name, 'codebase_lookup');
  assert.deepEqual(tool.parameters.required, ['question']);
  assert.equal(tool.parameters.properties.question.type, 'string');

  const context = {
    isProjectTrusted: () => false,
    sessionManager: { getSessionFile: () => undefined },
  };
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
