/**
 * Variable scope merge + `{{var}}` substitution (PLAN.md, "Variables").
 * v0.1 precedence: active environment > global. Keys are case-sensitive;
 * disabled variables never resolve; recursive values resolve to a maximum
 * depth of 5; cycles leave the cycling placeholder verbatim and report it.
 */

export interface Variable {
  key: string;
  value: string;
  enabled: boolean;
  secret: boolean;
}

export interface VariableSources {
  environment: Variable[];
  globals: Variable[];
}

export interface ResolvedVariable {
  value: string;
  source: 'environment' | 'global';
  secret: boolean;
}

export type ResolvedScope = Map<string, ResolvedVariable>;

const VAR_PATTERN_SOURCE = String.raw`\{\{\s*([\w.-]+)\s*\}\}`;
const MAX_DEPTH = 5;

/** Fresh regex per use — a shared global regex is not reentrant. */
function varPattern(): RegExp {
  return new RegExp(VAR_PATTERN_SOURCE, 'g');
}

/** Merge sources with precedence environment > global; disabled skipped. */
export function buildScope(sources: VariableSources): ResolvedScope {
  const scope: ResolvedScope = new Map();
  for (const variable of sources.globals) {
    if (variable.enabled) {
      scope.set(variable.key, {
        value: variable.value,
        source: 'global',
        secret: variable.secret,
      });
    }
  }
  for (const variable of sources.environment) {
    if (variable.enabled) {
      scope.set(variable.key, {
        value: variable.value,
        source: 'environment',
        secret: variable.secret,
      });
    }
  }
  return scope;
}

/**
 * Substitute `{{name}}` placeholders in `text`. Values are resolved
 * recursively to MAX_DEPTH; unknown names, cycles, and depth overflows leave
 * the placeholder verbatim and report the name in `unresolved` (deduplicated,
 * order of first appearance).
 */
export function substitute(
  text: string,
  scope: ResolvedScope,
): { output: string; unresolved: string[] } {
  const unresolved: string[] = [];
  const seen = new Set<string>();
  const report = (name: string): void => {
    if (!seen.has(name)) {
      seen.add(name);
      unresolved.push(name);
    }
  };

  const expand = (input: string, depth: number, stack: ReadonlySet<string>): string =>
    input.replace(varPattern(), (match, name: string) => {
      const entry = scope.get(name);
      if (entry === undefined || stack.has(name) || depth >= MAX_DEPTH) {
        report(name);
        return match;
      }
      const nextStack = new Set(stack);
      nextStack.add(name);
      return expand(entry.value, depth + 1, nextStack);
    });

  return { output: expand(text, 0, new Set()), unresolved };
}

/**
 * Detect cycles / depth overflows in a scope without substituting — used for
 * preserve-mode warnings, where substitution is skipped but broken variables
 * must still surface. Returns the names whose expansion would hit a cycle or
 * exceed MAX_DEPTH (references to undefined variables are NOT reported here).
 */
export function analyzeScope(scope: ResolvedScope): { cycles: string[] } {
  const referencesOf = (value: string): string[] => {
    const names: string[] = [];
    const pattern = varPattern();
    let match: RegExpExecArray | null;
    while ((match = pattern.exec(value)) !== null) {
      names.push(match[1]);
    }
    return names;
  };

  const walk = (value: string, depth: number, stack: ReadonlySet<string>): boolean => {
    let broken = false;
    for (const ref of referencesOf(value)) {
      const entry = scope.get(ref);
      if (entry === undefined) {
        continue;
      }
      if (stack.has(ref) || depth >= MAX_DEPTH) {
        broken = true;
        continue;
      }
      const nextStack = new Set(stack);
      nextStack.add(ref);
      if (walk(entry.value, depth + 1, nextStack)) {
        broken = true;
      }
    }
    return broken;
  };

  const cycles: string[] = [];
  for (const [name, entry] of scope) {
    if (walk(entry.value, 1, new Set([name]))) {
      cycles.push(name);
    }
  }
  return { cycles };
}
