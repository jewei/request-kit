import { describe, expect, it } from 'vitest';
import {
  analyzeScope,
  buildScope,
  substitute,
  type VariableSources,
} from './resolve';

function sources(
  environment: [string, string, boolean?][] = [],
  globals: [string, string, boolean?][] = [],
): VariableSources {
  const toVars = (rows: [string, string, boolean?][]) =>
    rows.map(([key, value, enabled]) => ({
      key,
      value,
      enabled: enabled ?? true,
      secret: false,
    }));
  return { environment: toVars(environment), globals: toVars(globals) };
}

describe('buildScope', () => {
  it('applies precedence environment > global', () => {
    const scope = buildScope(
      sources([['host', 'env.example.com']], [['host', 'global.example.com']]),
    );
    expect(scope.get('host')?.value).toBe('env.example.com');
    expect(scope.get('host')?.source).toBe('environment');
  });

  it('skips disabled variables', () => {
    const scope = buildScope(sources([['a', '1', false]], [['b', '2', false]]));
    expect(scope.size).toBe(0);
  });

  it('keys are case-sensitive', () => {
    const scope = buildScope(sources([['Host', 'upper']], [['host', 'lower']]));
    expect(scope.get('Host')?.value).toBe('upper');
    expect(scope.get('host')?.value).toBe('lower');
  });
});

describe('substitute', () => {
  it('replaces placeholders and tolerates whitespace', () => {
    const scope = buildScope(sources([['baseUrl', 'https://x.dev']]));
    expect(substitute('{{baseUrl}}/api', scope).output).toBe('https://x.dev/api');
    expect(substitute('{{ baseUrl }}/api', scope).output).toBe('https://x.dev/api');
  });

  it('reports unknown names once, in first-appearance order', () => {
    const { output, unresolved } = substitute('{{b}}/{{a}}/{{b}}', buildScope(sources()));
    expect(output).toBe('{{b}}/{{a}}/{{b}}');
    expect(unresolved).toEqual(['b', 'a']);
  });

  it('resolves recursively to depth 5', () => {
    const scope = buildScope(
      sources([
        ['v1', '{{v2}}'],
        ['v2', '{{v3}}'],
        ['v3', '{{v4}}'],
        ['v4', '{{v5}}'],
        ['v5', 'end'],
      ]),
    );
    expect(substitute('{{v1}}', scope).output).toBe('end');
  });

  it('cuts off past the depth limit and reports it', () => {
    const scope = buildScope(
      sources([
        ['d1', '{{d2}}'],
        ['d2', '{{d3}}'],
        ['d3', '{{d4}}'],
        ['d4', '{{d5}}'],
        ['d5', '{{d6}}'],
        ['d6', 'too deep'],
      ]),
    );
    const { output, unresolved } = substitute('{{d1}}', scope);
    expect(output).toBe('{{d6}}');
    expect(unresolved).toContain('d6');
  });

  it('detects cycles without hanging', () => {
    const scope = buildScope(
      sources([
        ['ping', '{{pong}}'],
        ['pong', '{{ping}}'],
      ]),
    );
    const { output, unresolved } = substitute('{{ping}}', scope);
    expect(output).toBe('{{ping}}');
    expect(unresolved).toContain('ping');
  });
});

describe('analyzeScope', () => {
  it('flags cycles and depth overflows without substituting', () => {
    const cyclic = buildScope(
      sources([
        ['a', '{{b}}'],
        ['b', '{{a}}'],
        ['ok', 'plain'],
      ]),
    );
    const { cycles } = analyzeScope(cyclic);
    expect(cycles).toContain('a');
    expect(cycles).toContain('b');
    expect(cycles).not.toContain('ok');
  });

  it('does not report references to undefined variables', () => {
    const scope = buildScope(sources([['a', '{{missing}}']]));
    expect(analyzeScope(scope).cycles).toEqual([]);
  });
});
