
//! The VFS trait/interface

use vnode::VNode;
use base::errno;
use std::borrow::Borrow;
use base::errno::{KResult};

pub trait FileSystem {
    type Real: VNode<Real=Self::Real, Res=Self::Node>;
    type Node: Borrow<Self::Real> + Clone;
    fn get_type(&self) -> &'static str;
    fn get_fs_root(&self) -> Self::Node;

    fn dir_namev<'a>(&self, name: &'a str, base: Self::Node) -> KResult<(Self::Node, &'a str)> {
        let name = trim_name(name);
        match name {
            "" => Err(errno::ENOENT),
            "/" => Ok((self.get_fs_root(), ".")),
            "./" => Ok((base, ".")),
            _ if name.starts_with("/") => self.dir_namev(&name[1..], self.get_fs_root()),
            _ => {
                // This goes on the end so we take trailing '/' as '/.'
                let end = if name.ends_with("/") { Some("." as &'a str) } else { None };
                let mut cp = base;
                let mut next = name;
                // We go through the split up name, which might have '.' appended to it.
                for n in name.split("/").chain(end.into_iter()) {
                    match n {
                        "" => {}, // A repeated '/', ignore it
                        "." => { next = n; }, // A './' ignore it.
                        _ => {
                            cp = try!({let x : &Self::Real = cp.borrow(); x.lookup(next)});
                            next = n;
                        },
                    }
                }
                Ok((cp, next))
            }
        }
    }
    fn open_namev(&self, name: &str, create: bool, mut base: Self::Node) -> KResult<Self::Node> {
        let (parent, fname) = try!(self.dir_namev(name, base));
        let parent : &Self::Real = parent.borrow();
        parent.lookup(fname).or_else(|err| { if err == errno::ENOENT && create { parent.create(fname) } else { Err(err) } })
    }
}

/// Removes repeated leading & trailing '/' from pathname
fn trim_name(n: &str) -> &str {
    let mut name = n;
    // 's:/+$:/:'
    while name.ends_with("//") { name = &name[..name.len() - 1]; }
    // 's:^/+:/:'
    while name.starts_with("//") { name = &name[1..]; }
    // 's:^(\./+)+::'
    while name.starts_with("./") { name = &name[1..].trim_left_matches("/"); }
    return name;
}
