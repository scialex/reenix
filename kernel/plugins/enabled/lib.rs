#![crate_type="dylib"]
#![crate_name="enabled"]
#![feature(plugin_registrar)]

#![doc = "A crate defining an is_enabled macro"]
extern crate syntax;
extern crate rustc;

use syntax::codemap::Span;
use syntax::parse::token;
use syntax::ast::{TokenTree, TtToken};
use syntax::ext::cfg;
use syntax::ext::base::{ExtCtxt, MacResult, DummyResult};
use rustc::plugin::Registry;


fn expand_is_enabled(cx: &mut ExtCtxt, sp: Span, args: &[TokenTree]) -> Box<MacResult + 'static> {
    let (title, name) = match args {
        [TtToken(_, token::Ident(title, _)), TtToken(_, token::RArrow), TtToken(_, token::Ident(name, _))] =>
            (token::get_ident(title), token::get_ident(name)),
        _ => {
            cx.span_err(sp, "Argument should be 'module_name->option_name' to check for cfg of 'Nmodule_name_option_name'");
            return DummyResult::any(sp);
        }
    };
    let mut check_name = String::from_str("N");
    check_name.push_str(title.get());
    check_name.push('_');
    check_name.push_str(name.get());
    let outtok = token::gensym_ident(check_name.as_slice());
    let toktree = [TtToken(sp, token::Ident(outtok, token::Plain))];
    cfg::expand_cfg(cx, sp, &toktree)
}

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_macro("is_enabled", expand_is_enabled);
}
