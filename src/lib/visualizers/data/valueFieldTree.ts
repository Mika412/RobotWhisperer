import type { Value } from "$lib/core/types";

export interface ValueField {
  path: string;
  label: string;
  typeLabel: string;
  plottable: boolean;
  children?: ValueField[];
}

const NUMERIC = new Set(["int", "uint", "f32", "f64", "bool"]);

function typeLabel(value: Value): string {
  switch (value.kind) {
    case "array":
      return `${value.value.length > 0 ? kindLabel(value.value[0]) : "empty"}[${value.value.length}]`;
    case "struct":
      return `{${Object.keys(value.value).length}}`;
    default:
      return kindLabel(value);
  }
}

function kindLabel(value: Value): string {
  return value.kind;
}

function build(value: Value, path: string, label: string): ValueField {
  if (value.kind === "struct") {
    return {
      path,
      label,
      typeLabel: typeLabel(value),
      plottable: false,
      children: Object.entries(value.value).map(([key, child]) =>
        build(child, path ? `${path}.${key}` : key, key),
      ),
    };
  }
  if (value.kind === "array") {
    return {
      path,
      label,
      typeLabel: typeLabel(value),
      plottable: false,
      children: value.value.map((child, index) => build(child, `${path}[${index}]`, `[${index}]`)),
    };
  }
  return { path, label, typeLabel: typeLabel(value), plottable: NUMERIC.has(value.kind) };
}

export function buildValueTree(value: unknown): ValueField[] {
  if (!value || typeof value !== "object") return [];
  const root = value as Value;
  if (root.kind !== "struct") return [build(root, "", "value")];
  return Object.entries(root.value).map(([key, child]) => build(child, key, key));
}

export function shapeSignature(value: unknown, depth = 0): string {
  if (depth > 8 || !value || typeof value !== "object") return "";
  const node = value as Value;
  if (node.kind === "struct") {
    return `{${Object.entries(node.value)
      .map(([key, child]) => `${key}:${shapeSignature(child, depth + 1)}`)
      .join(",")}}`;
  }
  if (node.kind === "array") {
    return `[${node.value.length}:${node.value.length > 0 ? shapeSignature(node.value[0], depth + 1) : ""}]`;
  }
  return node.kind;
}
