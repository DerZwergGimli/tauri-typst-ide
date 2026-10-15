import { computed, type Ref } from "vue";
import type { FileNode } from "@/types/models";

export interface FlatNode {
  node: FileNode;
  depth: number;
}

export function useFileTree(
  nodes: Ref<FileNode[]>,
  expanded: Ref<Set<string>>,
  focusedPath: Ref<string | null>,
) {
  const visible = computed<FlatNode[]>(() => {
    const out: FlatNode[] = [];
    const walk = (list: FileNode[], depth: number) => {
      for (const node of list) {
        out.push({ node, depth });
        if (node.kind === "dir" && expanded.value.has(node.path) && node.children) {
          walk(node.children, depth + 1);
        }
      }
    };
    walk(nodes.value, 0);
    return out;
  });

  function expand(path: string) {
    expanded.value.add(path);
  }

  function collapse(path: string) {
    expanded.value.delete(path);
  }

  function toggle(path: string) {
    if (expanded.value.has(path)) collapse(path);
    else expand(path);
  }

  function moveFocus(delta: number) {
    const list = visible.value;
    if (list.length === 0) return;
    const idx = list.findIndex((f) => f.node.path === focusedPath.value);
    const next = Math.min(Math.max(idx + delta, 0), list.length - 1);
    focusedPath.value = list[next]!.node.path;
  }

  function focusFirst() {
    const first = visible.value[0];
    if (first) focusedPath.value = first.node.path;
  }

  function focusLast() {
    const last = visible.value[visible.value.length - 1];
    if (last) focusedPath.value = last.node.path;
  }

  /** Move focus to the first visible child, or expand if collapsed. */
  function moveRight(path: string, kind: string) {
    if (kind !== "dir") return;
    if (!expanded.value.has(path)) {
      expand(path);
    } else {
      moveFocus(1);
    }
  }

  /** Collapse, or move focus to the parent dir. */
  function moveLeft(path: string) {
    if (expanded.value.has(path)) {
      collapse(path);
      return;
    }
    const list = visible.value;
    const idx = list.findIndex((f) => f.node.path === path);
    if (idx <= 0) return;
    const depth = list[idx]!.depth;
    for (let i = idx - 1; i >= 0; i--) {
      if (list[i]!.depth < depth) {
        focusedPath.value = list[i]!.node.path;
        return;
      }
    }
  }

  function findNode(path: string): FileNode | null {
    let found: FileNode | null = null;
    const walk = (list: FileNode[]) => {
      for (const node of list) {
        if (node.path === path) {
          found = node;
          return;
        }
        if (node.children) walk(node.children);
      }
    };
    walk(nodes.value);
    return found;
  }

  return {
    visible,
    expand,
    collapse,
    toggle,
    moveFocus,
    focusFirst,
    focusLast,
    moveRight,
    moveLeft,
    findNode,
  };
}
