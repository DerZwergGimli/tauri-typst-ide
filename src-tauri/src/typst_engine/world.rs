use std::path::Path;
use std::sync::{Arc, LazyLock};

use typst::diag::FileResult;
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::datetime::Time;
use typst_kit::downloader::SystemDownloader;
use typst_kit::files::{FileStore, FsRoot, SystemFiles};
use typst_kit::fonts::{self, FontStore};
use typst_kit::packages::SystemPackages;

use crate::error::{AppError, AppResult};

/// Font store shared across compilations. Building it scans system fonts, which
/// is expensive, so we do it exactly once per app run.
pub fn shared_fonts() -> Arc<LazyLock<FontStore>> {
    static FONTS: LazyLock<Arc<LazyLock<FontStore>>> = LazyLock::new(|| {
        Arc::new(LazyLock::new(|| {
            let mut store = FontStore::new();
            store.extend(fonts::system());
            store.extend(fonts::embedded());
            store
        }))
    });
    FONTS.clone()
}

fn make_system_files(root: &Path) -> SystemFiles {
    let downloader =
        SystemDownloader::new(concat!("tauri-typst-ide/", env!("CARGO_PKG_VERSION")));
    SystemFiles::new(FsRoot::new(root.to_path_buf()), SystemPackages::new(downloader))
}

/// The Typst [`World`] for a single project. Cheap to construct; the expensive
/// font store is shared. Construct a fresh world per compilation so file
/// contents and dependency tracking are always current.
pub struct ProjectWorld {
    main: FileId,
    library: LazyHash<Library>,
    fonts: Arc<LazyLock<FontStore>>,
    files: FileStore<SystemFiles>,
    now: Time,
}

impl ProjectWorld {
    pub fn new(root: &Path, main: &Path, fonts: Arc<LazyLock<FontStore>>) -> AppResult<Self> {
        let main = main_id(root, main)?;
        Ok(Self {
            main,
            library: LazyHash::new(Library::default()),
            fonts,
            files: FileStore::new(make_system_files(root)),
            now: Time::system(),
        })
    }

    /// A display name for a file id: project-relative path or `@ns/name:version`.
    pub fn name(&self, id: FileId) -> String {
        match id.root() {
            VirtualRoot::Project => id.vpath().get_without_slash().into(),
            VirtualRoot::Package(pkg) => {
                format!("@{}/{}:{}", pkg.namespace, pkg.name, pkg.version)
            }
        }
    }
}

impl World for ProjectWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.fonts.book()
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<typst::syntax::Source> {
        self.files.source(id)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files.file(id)
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.font(index)
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        self.now.today(offset)
    }
}

/// Build the main-file `FileId` for a project root.
pub fn main_id(root: &Path, main: &Path) -> AppResult<FileId> {
    let vpath = VirtualPath::virtualize(root, main)
        .map_err(|e| AppError::InvalidPath(format!("main file outside project root: {e}")))?;
    Ok(RootedPath::new(VirtualRoot::Project, vpath).intern())
}
