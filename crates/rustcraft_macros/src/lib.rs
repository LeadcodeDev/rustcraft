use proc_macro::TokenStream;
use quote::quote;
use syn::{ImplItem, ItemImpl, parse_macro_input};

/// Maps `#[Event::Xxx]` attribute names to `RustcraftPlugin` trait method names.
fn event_to_trait_method(event_name: &str) -> Option<&'static str> {
    match event_name {
        "BlockPlaced" => Some("on_block_placed"),
        "BlockRemoved" => Some("on_block_removed"),
        "PlayerMoved" => Some("on_player_moved"),
        "GameModeChanged" => Some("on_gamemode_changed"),
        "InventoryPickedUp" => Some("on_inventory_picked_up"),
        "InventoryDropped" => Some("on_inventory_dropped"),
        "ItemDroppedToWorld" => Some("on_item_dropped_to_world"),
        "ItemsCollected" => Some("on_items_collected"),
        _ => None,
    }
}

/// Proc-macro attribute that generates a `RustcraftPlugin` trait implementation.
///
/// # Usage
/// ```ignore
/// #[rustcraft_plugin]
/// impl MyPlugin {
///     #[Event::PlayerMoved]
///     fn on_move(&self, event: &PlayerMoved) {
///         info!("Player moved to {:?}", event.player);
///     }
/// }
/// ```
///
/// This generates:
/// - The original `impl MyPlugin` block (with event attributes stripped)
/// - An `impl RustcraftPlugin for MyPlugin` that delegates to the annotated methods
#[proc_macro_attribute]
pub fn craft_plugin(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemImpl);
    let self_ty = &input.self_ty;

    let mut trait_methods = Vec::new();

    for item in &mut input.items {
        let ImplItem::Fn(method) = item else {
            continue;
        };

        // Find and remove #[Event::Xxx] attributes
        let mut event_name = None;
        method.attrs.retain(|attr| {
            let segments: Vec<_> = attr.path().segments.iter().collect();
            if segments.len() == 2 && segments[0].ident == "Event" {
                event_name = Some(segments[1].ident.to_string());
                return false; // remove this attribute
            }
            true // keep other attributes
        });

        let Some(name) = event_name else {
            continue;
        };

        let Some(trait_method_name) = event_to_trait_method(&name) else {
            continue;
        };

        let trait_method_ident = syn::Ident::new(trait_method_name, method.sig.ident.span());
        let user_method_ident = &method.sig.ident;

        // Extract the event type from the second parameter: &self, event: &EventType
        let event_type = method
            .sig
            .inputs
            .iter()
            .nth(1)
            .and_then(|arg| {
                if let syn::FnArg::Typed(pat_type) = arg {
                    Some(&pat_type.ty)
                } else {
                    None
                }
            })
            .expect("Event handler must have a second parameter for the event type");

        trait_methods.push(quote! {
            fn #trait_method_ident(&self, event: #event_type) {
                self.#user_method_ident(event)
            }
        });
    }

    let expanded = quote! {
        #input

        impl crate::events::RustcraftPlugin for #self_ty {
            #(#trait_methods)*
        }
    };

    expanded.into()
}
