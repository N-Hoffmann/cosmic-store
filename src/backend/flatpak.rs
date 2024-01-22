use appstream::{xmltree, Collection, ParseError};
use cosmic::widget;
use flate2::read::GzDecoder;
use libflatpak::{gio::Cancellable, prelude::*, Installation, RefKind};
use std::{collections::HashMap, error::Error};

use super::{Backend, Package};

pub struct Flatpak {
    inst: Installation,
}

impl Flatpak {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        //TODO: should we support system installations?
        let inst = Installation::new_user(Cancellable::NONE)?;
        {
            let start = std::time::Instant::now();
            match inst.list_remotes(Cancellable::NONE) {
                Ok(remotes) => for remote in remotes {
                    let remote_name = match remote.name() {
                        Some(some) => some,
                        None => {
                            continue;
                        }
                    };
                    match inst.list_remote_refs_sync(&remote_name, Cancellable::NONE) {
                        Ok(refs) => {
                            println!("remote {} has {} refs", remote_name, refs.len());
                        },
                        Err(err) => {
                            log::warn!("list remote {} refs error: {}", remote_name, err);
                        }
                    }
                },
                Err(err) => {
                    log::warn!("list remotes error: {}", err);
                }
            }
            let duration = start.elapsed();
            log::info!("listed remote refs in {:?}", duration);
        }
        Ok(Self { inst })
    }
}

impl Backend for Flatpak {
    fn installed(&self) -> Result<Vec<Package>, Box<dyn Error>> {
        //TODO: show non-desktop items?
        let installed = self
            .inst
            .list_installed_refs_by_kind(RefKind::App, Cancellable::NONE)?;
        let mut packages = Vec::new();
        for r in installed.iter() {
            if let Some(id) = r.name() {
                let mut extra = HashMap::new();
                if let Some(arch) = r.arch() {
                    extra.insert("arch".to_string(), arch.to_string());
                }
                if let Some(branch) = r.branch() {
                    extra.insert("branch".to_string(), branch.to_string());
                }
                packages.push(Package {
                    id: id.to_string(),
                    //TODO: get icon from appstream data?
                    icon: widget::icon::from_name(id.to_string()).size(128).handle(),
                    name: r.appdata_name().unwrap_or(id).to_string(),
                    version: r.appdata_version().unwrap_or_default().to_string(),
                    extra,
                })
            }
        }
        Ok(packages)
    }

    fn appstream(&self, package: &Package) -> Result<Collection, Box<dyn Error>> {
        let r = self.inst.installed_ref(
            RefKind::App,
            &package.id,
            package.extra.get("arch").map(|x| x.as_str()),
            package.extra.get("branch").map(|x| x.as_str()),
            Cancellable::NONE,
        )?;
        let bytes = r.load_appdata(Cancellable::NONE)?;
        let mut gz = GzDecoder::new(&*bytes);
        let element = xmltree::Element::parse(&mut gz)?;
        let collection = Collection::try_from(&element).map_err(ParseError::from)?;
        Ok(collection)
    }
}
