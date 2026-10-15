import {
  File as FileIcon,
  FileCode2,
  FileText,
  Folder,
  FolderOpen,
  Image,
} from "@lucide/vue";
import type { Component } from "vue";

const EXT_ICONS: Record<string, Component> = {
  typ: FileText,
  md: FileText,
  js: FileCode2,
  ts: FileCode2,
  json: FileCode2,
  css: FileCode2,
  scss: FileCode2,
  html: FileCode2,
  rs: FileCode2,
  py: FileCode2,
  toml: FileCode2,
  yaml: FileCode2,
  yml: FileCode2,
  png: Image,
  jpg: Image,
  jpeg: Image,
  gif: Image,
  svg: Image,
  webp: Image,
};

export function iconForFile(name: string): Component {
  const ext = name.includes(".") ? name.split(".").pop()!.toLowerCase() : "";
  return EXT_ICONS[ext] ?? FileIcon;
}

export { Folder, FolderOpen };
