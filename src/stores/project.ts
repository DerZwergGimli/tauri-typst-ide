import { computed, ref } from "vue";
import { defineStore } from "pinia";
import type { FileNode, ProjectInfo, RecentProject } from "@/types/models";
import { useEditorStore } from "@/stores/editor";

const MAX_RECENTS = 10;

export const useProjectStore = defineStore("project", () => {
  const root = ref<string | null>(null);
  const name = ref<string | null>(null);
  const mainFile = ref<string | null>(null);
  const typFiles = ref<string[]>([]);
  const tree = ref<FileNode | null>(null);
  const treeLoading = ref(false);
  const modalOpen = ref(false);
  const recents = ref<RecentProject[]>([]);

  const hasProject = computed(() => root.value !== null);

  let settingsStorePromise: Promise<
    import("@tauri-apps/plugin-store").Store
  > | null = null;

  async function settingsStore() {
    if (!settingsStorePromise) {
      const { load } = await import("@tauri-apps/plugin-store");
      settingsStorePromise = load("app-settings.json");
    }
    return settingsStorePromise;
  }

  async function loadRecents() {
    try {
      const store = await settingsStore();
      const raw = await store.get<RecentProject[]>("recents");
      recents.value = Array.isArray(raw) ? raw : [];
      // validate existence in the background
      const { invoke } = await import("@tauri-apps/api/core");
      const checked = await Promise.all(
        recents.value.map(async (r) => ({
          r,
          exists: await invoke<boolean>("path_exists", { path: r.path }),
        })),
      );
      recents.value = checked.filter((c) => c.exists).map((c) => c.r);
    } catch {
      recents.value = [];
    }
  }

  async function addRecent(info: ProjectInfo) {
    const entry: RecentProject = {
      path: info.root,
      name: info.name,
      last_opened: new Date().toISOString(),
    };
    const next = [
      entry,
      ...recents.value.filter((r) => r.path !== entry.path),
    ].slice(0, MAX_RECENTS);
    recents.value = next;
    try {
      const store = await settingsStore();
      await store.set("recents", next);
    } catch {
      // non-fatal
    }
  }

  async function removeRecent(path: string) {
    recents.value = recents.value.filter((r) => r.path !== path);
    try {
      const store = await settingsStore();
      await store.set("recents", recents.value);
    } catch {
      // non-fatal
    }
  }

  /** Per-project expanded folder paths, persisted across restarts. */
  async function loadExpandedDirs(projectRoot: string): Promise<string[]> {
    try {
      const store = await settingsStore();
      const raw = await store.get<string[]>(`expanded:${projectRoot}`);
      return Array.isArray(raw) ? raw : [];
    } catch {
      return [];
    }
  }

  async function saveExpandedDirs(projectRoot: string, dirs: string[]) {
    try {
      const store = await settingsStore();
      await store.set(`expanded:${projectRoot}`, dirs);
    } catch {
      // non-fatal
    }
  }

  async function refreshTree() {
    treeLoading.value = true;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      tree.value = await invoke<FileNode>("read_dir_tree");
    } finally {
      treeLoading.value = false;
    }
  }

  async function openProject(path: string) {
    const { invoke } = await import("@tauri-apps/api/core");
    const info = await invoke<ProjectInfo>("open_project", { path });
    applyProjectInfo(info);
    modalOpen.value = false;
    await addRecent(info);
    await refreshTree();
  }

  async function createProject(parent: string, projectName: string) {
    const { invoke } = await import("@tauri-apps/api/core");
    const info = await invoke<ProjectInfo>("create_project", {
      parent,
      name: projectName,
    });
    applyProjectInfo(info);
    modalOpen.value = false;
    await addRecent(info);
    await refreshTree();
  }

  async function closeProject() {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("close_project");
    root.value = null;
    name.value = null;
    mainFile.value = null;
    typFiles.value = [];
    tree.value = null;
    useEditorStore().reset();
  }

  async function setMainFile(path: string) {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("set_main_file", { relPath: path });
    mainFile.value = path;
  }

  function applyProjectInfo(info: ProjectInfo) {
    // Discard any buffers from the previously open project.
    useEditorStore().reset();
    root.value = info.root;
    name.value = info.name;
    mainFile.value = info.main_file;
    typFiles.value = info.typ_files;
  }

  return {
    root,
    name,
    mainFile,
    typFiles,
    tree,
    treeLoading,
    modalOpen,
    recents,
    hasProject,
    loadRecents,
    removeRecent,
    loadExpandedDirs,
    saveExpandedDirs,
    refreshTree,
    openProject,
    createProject,
    closeProject,
    setMainFile,
  };
});
