import type { Value } from "$lib/core/types";

export type PathStep = { kind: "key"; name: string } | { kind: "index"; index: number };

const TOKEN = /[A-Za-z_][A-Za-z0-9_]*|\[\d+\]/g;

export function parsePath(path: string): PathStep[] {
  if (!path) return [];
  const tokens = path.replace(/\./g, " ").match(TOKEN) ?? [];
  return tokens.map((token) =>
    token.startsWith("[")
      ? { kind: "index", index: Number(token.slice(1, -1)) }
      : { kind: "key", name: token },
  );
}

export function readValue(root: unknown, steps: PathStep[]): Value | undefined {
  let cursor: unknown = root;
  for (const step of steps) {
    if (!cursor || typeof cursor !== "object") return undefined;
    const node = cursor as Value;
    if (step.kind === "key") {
      if (node.kind !== "struct") return undefined;
      cursor = node.value[step.name];
    } else {
      if (node.kind !== "array" || step.index < 0 || step.index >= node.value.length) {
        return undefined;
      }
      cursor = node.value[step.index];
    }
  }
  return cursor as Value | undefined;
}

function asNumber(value: Value | undefined): number | undefined {
  if (!value) return undefined;
  switch (value.kind) {
    case "int":
    case "uint":
    case "f32":
    case "f64":
      return value.value;
    case "bool":
      return value.value ? 1 : 0;
    default:
      return undefined;
  }
}

export function readNumber(root: unknown, steps: PathStep[]): number | undefined {
  return asNumber(readValue(root, steps));
}

export function readNumberArray(root: unknown, steps: PathStep[]): number[] | undefined {
  const node = readValue(root, steps);
  if (!node || node.kind !== "array") return undefined;
  return node.value.map((element) => asNumber(element) ?? NaN);
}
