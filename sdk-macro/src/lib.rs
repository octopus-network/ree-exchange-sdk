use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::collections::BTreeMap;
use syn::{
    parse_macro_input, parse_quote, visit_mut::VisitMut, Attribute, Ident, ItemFn, ItemMod, Meta,
};

#[derive(Clone)]
struct CanisterVisitor {
    actions: BTreeMap<String, (String, bool)>,
    pools: Option<Ident>,
    hook_present: bool,
}

impl CanisterVisitor {
    fn new() -> Self {
        CanisterVisitor {
            actions: BTreeMap::new(),
            pools: None,
            hook_present: false,
        }
    }

    fn resolve_pools(&mut self, ty: &syn::ItemStruct) {
        let mark_pools = ty.attrs.iter().find(|a| a.path().is_ident("pools"));
        if mark_pools.is_none() {
            return;
        }
        if self.pools.is_some() {
            panic!("Only one struct can have the #[pools] attribute");
        }
        self.pools = Some(ty.ident.clone());
    }

    fn resolve_action(&mut self, attr: &Attribute, func: &ItemFn) {
        let is_action = attr.path().is_ident("action");
        if !is_action {
            return;
        }
        if let Meta::Path(_) = &attr.meta {
            self.actions.insert(
                func.sig.ident.to_string(),
                (func.sig.ident.to_string(), func.sig.asyncness.is_some()),
            );
        } else if let Meta::List(meta_list) = &attr.meta {
            let exp = meta_list.tokens.clone().into_iter().collect::<Vec<_>>();
            if exp.is_empty() {
                let action = func.sig.ident.to_string();
                self.actions.insert(
                    action,
                    (func.sig.ident.to_string(), func.sig.asyncness.is_some()),
                );
            } else if exp.len() == 1 {
                if let proc_macro2::TokenTree::Literal(lit) = &exp[0] {
                    let action = lit.to_string().replace('"', "");
                    self.actions.insert(
                        action,
                        (func.sig.ident.to_string(), func.sig.asyncness.is_some()),
                    );
                } else {
                    panic!("Expected #[action(\"...\")] attribute");
                }
            } else if exp.len() == 3 {
                let b0 = matches!(exp[0].clone(), proc_macro2::TokenTree::Ident(ident) if ident.to_string() == "name");
                let b1 = matches!(exp[1].clone(), proc_macro2::TokenTree::Punct(punct) if punct.as_char() == '=');
                if !(b0 && b1) {
                    panic!("Expected #[action(name = \"...\")] attribute");
                }
                if let proc_macro2::TokenTree::Literal(lit) = &exp[2] {
                    let action = lit.to_string().replace('"', "");
                    self.actions.insert(
                        action,
                        (func.sig.ident.to_string(), func.sig.asyncness.is_some()),
                    );
                } else {
                    panic!("Expected #[action(name = \"...\")] attribute");
                }
            } else {
                panic!("Unexpected tokens in #[action] macro");
            }
        } else {
            panic!("Expected `#[action(\"..\")]` or `#[action(name = \"..\")]` or `#[action]` attribute");
        }
    }
}

impl VisitMut for CanisterVisitor {
    fn visit_item_fn_mut(&mut self, item: &mut syn::ItemFn) {
        for attr in item.attrs.iter() {
            self.resolve_action(&attr, item);
        }
        syn::visit_mut::visit_item_fn_mut(self, item);
    }

    fn visit_item_struct_mut(&mut self, item: &mut syn::ItemStruct) {
        self.resolve_pools(item);
        syn::visit_mut::visit_item_struct_mut(self, item);
    }

    fn visit_item_impl_mut(&mut self, item: &mut syn::ItemImpl) {
        if let Some(_attr) = item.attrs.iter().find(|a| a.path().is_ident("hook")) {
            self.hook_present = true;
        }
        syn::visit_mut::visit_item_impl_mut(self, item);
    }
}

/// REE exchange entrypoint.
#[proc_macro_attribute]
pub fn exchange(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input_mod = parse_macro_input!(item as ItemMod);
    let mut visitor = CanisterVisitor::new();
    visitor.visit_item_mod_mut(&mut input_mod);
    if visitor.pools.is_none() {
        panic!("#[pools] not found within the exchange mod");
    }
    let pools = visitor.pools.clone().unwrap();
    if let Some((_, ref mut items)) = input_mod.content {
        let branch = visitor
            .actions
            .iter()
            .map(|(action, (func, is_async))| {
                let call = format_ident!("{}", func);
                if *is_async {
                    quote! { #action => #call(args).await, }
                } else {
                    quote! { #action => #call(args), }
                }
            })
            .collect::<Vec<_>>();

        if !visitor.hook_present {
            items.push(parse_quote! {
                impl ::ree_exchange_sdk::exchange_interfaces::Hook for #pools {}
            });
        }

        items.push(parse_quote! {
            impl ::ree_exchange_sdk::exchange_interfaces::PoolStorageAccess<#pools> for #pools {
                fn get(address: &::std::string::String) -> ::std::option::Option<::ree_exchange_sdk::exchange_interfaces::Pool<<#pools as ::ree_exchange_sdk::exchange_interfaces::Pools>::State>> {
                    self::__CURRENT_POOLS.with_borrow(|p| p.get(address))
                }

                fn insert(pool: ::ree_exchange_sdk::exchange_interfaces::Pool<<#pools as ::ree_exchange_sdk::exchange_interfaces::Pools>::State>) {
                    self::__CURRENT_POOLS.with_borrow_mut(|p| {
                        p.insert(pool.metadata().address.clone(), pool);
                    });
                }

                fn remove(address: &::std::string::String) -> ::std::option::Option<::ree_exchange_sdk::exchange_interfaces::Pool<<#pools as ::ree_exchange_sdk::exchange_interfaces::Pools>::State>> {
                    self::__CURRENT_POOLS.with_borrow_mut(|p| {
                        p.remove(address)
                    })
                }

                fn iter() -> ::ree_exchange_sdk::exchange_interfaces::iter::PoolIterator<#pools> {
                    ::ree_exchange_sdk::exchange_interfaces::iterator::<#pools>()
                }
            }
        });

        items.push(parse_quote! {
            #[::ic_cdk::update]
            pub async fn execute_tx(args: ::ree_exchange_sdk::exchange_interfaces::ExecuteTxArgs) -> ::core::result::Result<String, String> {
                ::ree_exchange_sdk::exchange_interfaces::ensure_access::<#pools>()?;
                let ::ree_exchange_sdk::exchange_interfaces::ExecuteTxArgs {
                    psbt_hex,
                    txid,
                    intention_set,
                    intention_index,
                    zero_confirmed_tx_queue_length,
                } = args;
                let ::ree_exchange_sdk::Intention {
                    exchange_id,
                    action,
                    action_params,
                    pool_address,
                    nonce,
                    pool_utxo_spent,
                    pool_utxo_received,
                    input_coins,
                    output_coins,
                } = &intention_set.intentions[intention_index as usize];
                let pool_address = pool_address.clone();
                let _guard = self::__ExecuteTxGuard::new(pool_address.clone())
                    .ok_or(format!("Pool {} is being executed", pool_address))?;
                let txid = txid.clone();
                let action = action.clone();
                let args = ::ree_exchange_sdk::exchange_interfaces::ExecuteTxArgs {
                    psbt_hex,
                    txid,
                    intention_set,
                    intention_index,
                    zero_confirmed_tx_queue_length,
                };
                let result: ::ree_exchange_sdk::exchange_interfaces::ActionResult::<<#pools as ::ree_exchange_sdk::exchange_interfaces::Pools>::State> = match action.as_str() {
                    #(#branch)*
                    _ => ::ree_exchange_sdk::exchange_interfaces::ActionResult::<<#pools as ::ree_exchange_sdk::exchange_interfaces::Pools>::State>::Err(format!("Unknown action: {}", action)),
                };
                match result {
                    ::ree_exchange_sdk::exchange_interfaces::ActionResult::<<#pools as ::ree_exchange_sdk::exchange_interfaces::Pools>::State>::Ok(r) => {
                        self::__CURRENT_POOLS.with_borrow_mut(|pools| {
                            if let Some(mut pool) = pools.get(&pool_address) {
                                pool.states_mut().push(r);
                                pools.insert(pool_address.clone(), pool);
                                ::core::result::Result::<(), String>::Ok(())
                            } else {
                                ::core::result::Result::<(), String>::Err(format!("Pool {} not found", pool_address))
                            }
                        })?;
                        self::__TX_RECORDS.with_borrow_mut(|m| {
                            let mut record = m.get(&(txid.clone(), false)).unwrap_or_default();
                            if !record.pools.contains(&pool_address) {
                                record.pools.push(pool_address.clone());
                            }
                            m.insert((txid.clone(), false), record);
                        });
                        ::core::result::Result::<String, String>::Ok(txid.to_string())
                    }
                    ::ree_exchange_sdk::exchange_interfaces::ActionResult::<<#pools as ::ree_exchange_sdk::exchange_interfaces::Pools>::State>::Err(e) => {
                        ::core::result::Result::<String, String>::Err(e)
                    }
                }
            }
        });

        items.push(parse_quote! {
            #[::ic_cdk::query]
            pub fn get_pool_list() -> ::ree_exchange_sdk::exchange_interfaces::GetPoolListResponse {
                self::__CURRENT_POOLS.with_borrow(|pools| {
                    pools.iter()
                        .map(|e| e.into_pair())
                        .map(|(_, p)| p.get_pool_basic())
                        .collect::<Vec<_>>()
                })
            }
        });

        items.push(parse_quote! {
            #[::ic_cdk::query]
            pub fn get_pool_info(
                args: ::ree_exchange_sdk::exchange_interfaces::GetPoolInfoArgs,
            ) -> ::ree_exchange_sdk::exchange_interfaces::GetPoolInfoResponse {
                self::__CURRENT_POOLS.with_borrow(|pools| {
                    pools.get(&args.pool_address).map(|p| p.get_pool_info())
                })
            }
        });

        items.push(parse_quote! {
            #[::ic_cdk::update]
            pub fn rollback_tx(
                args: ::ree_exchange_sdk::exchange_interfaces::RollbackTxArgs,
            ) -> ::ree_exchange_sdk::exchange_interfaces::RollbackTxResponse {
                ::ree_exchange_sdk::exchange_interfaces::ensure_access::<#pools>()?;
                self::__TX_RECORDS.with_borrow_mut(|transactions| {
                    self::__CURRENT_POOLS.with_borrow_mut(|pools| {
                        ::ree_exchange_sdk::reorg::rollback_tx::<#pools>(transactions, pools, args)
                    })
                })
            }
        });

        items.push(parse_quote! {
            #[::ic_cdk::update]
            pub fn new_block(
                args: ::ree_exchange_sdk::exchange_interfaces::NewBlockArgs,
            ) -> ::ree_exchange_sdk::exchange_interfaces::NewBlockResponse {
                ::ree_exchange_sdk::exchange_interfaces::ensure_access::<#pools>()?;
                self::__TX_RECORDS.with_borrow_mut(|transactions| {
                    self::__CURRENT_POOLS.with_borrow_mut(|pools| {
                        self::__BLOCKS.with_borrow_mut(|blocks| {
                            ::ree_exchange_sdk::reorg::new_block::<#pools>(
                                blocks,
                                transactions,
                                pools,
                                args
                            )
                        })
                    })
                })
            }
        });

        items.push(parse_quote! {
            struct __ExecuteTxGuard(::std::string::String);
        });

        items.push(parse_quote! {
            impl __ExecuteTxGuard {
                pub fn new(pool_address: ::std::string::String) -> ::std::option::Option<Self> {
                    __GUARDS.with(|guards| {
                        if guards.borrow().contains(&pool_address) {
                            return None;
                        }
                        guards.borrow_mut().insert(pool_address.clone());
                        return Some(__ExecuteTxGuard(pool_address));
                    })
                }
            }
        });

        items.push(parse_quote! {
            impl ::std::ops::Drop for __ExecuteTxGuard {
                fn drop(&mut self) {
                    __GUARDS.with_borrow_mut(|guards| {
                        guards.remove(&self.0);
                    });
                }
            }
        });

        items.push(parse_quote! {
            thread_local! {
                static __MEMORY_MANAGER: ::core::cell::RefCell<
                    ::ic_stable_structures::memory_manager::MemoryManager<
                        ::ic_stable_structures::DefaultMemoryImpl
                    >
                > = ::core::cell::RefCell::new(
                    ::ic_stable_structures::memory_manager::MemoryManager::init(
                        <::ic_stable_structures::DefaultMemoryImpl as core::default::Default>::default()
                    )
                );
                static __GUARDS: ::core::cell::RefCell<::std::collections::HashSet<::std::string::String>> =
                    ::core::cell::RefCell::new(::std::collections::HashSet::new());
                static __BLOCKS: ::core::cell::RefCell<
                    ::ic_stable_structures::StableBTreeMap<
                        u32,
                        ::ree_exchange_sdk::exchange_interfaces::NewBlockInfo,
                        ::ic_stable_structures::memory_manager::VirtualMemory<::ic_stable_structures::DefaultMemoryImpl>
                    >
                > = ::core::cell::RefCell::new(
                    ::ic_stable_structures::StableBTreeMap::init(
                        __MEMORY_MANAGER.with(|m| m.borrow().get(::ic_stable_structures::memory_manager::MemoryId::new(
                            <#pools as ::ree_exchange_sdk::exchange_interfaces::Pools>::BLOCK_MEMORY
                        ))),
                    )
                );
                static __TX_RECORDS: ::core::cell::RefCell<
                    ::ic_stable_structures::StableBTreeMap<
                        (::ree_exchange_sdk::Txid, bool),
                        ::ree_exchange_sdk::TxRecord,
                        ::ic_stable_structures::memory_manager::VirtualMemory<::ic_stable_structures::DefaultMemoryImpl>
                    >
                > = ::core::cell::RefCell::new(
                    ::ic_stable_structures::StableBTreeMap::init(
                        __MEMORY_MANAGER.with(|m| m.borrow().get(::ic_stable_structures::memory_manager::MemoryId::new(
                            <#pools as ::ree_exchange_sdk::exchange_interfaces::Pools>::TRANSACTION_MEMORY
                        ))),
                    )
                );
                static __CURRENT_POOLS: ::core::cell::RefCell<
                    ::ic_stable_structures::StableBTreeMap<
                        ::std::string::String,
                        ::ree_exchange_sdk::exchange_interfaces::Pool<
                            <#pools as ::ree_exchange_sdk::exchange_interfaces::Pools>::State
                        >,
                        ::ic_stable_structures::memory_manager::VirtualMemory<::ic_stable_structures::DefaultMemoryImpl>
                    >
                > = ::core::cell::RefCell::new(
                    ::ic_stable_structures::StableBTreeMap::init(
                        __MEMORY_MANAGER.with(|m| m.borrow().get(::ic_stable_structures::memory_manager::MemoryId::new(
                            <#pools as ::ree_exchange_sdk::exchange_interfaces::Pools>::POOL_MEMORY
                        ))),
                    )
                );
            }
        });
    }
    quote! {
        #input_mod
    }
    .into()
}

/// Action entrypoint. The macro could be
/// ```#[action(name = "my_action")]``` or ```#[action("my_action")]``` or ```#[action]```.
/// The function shall have a single argument `ExecuteTxArgs` and return an `ActionResult<Pools::State>`
#[proc_macro_attribute]
pub fn action(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Pools definition
#[proc_macro_attribute]
pub fn pools(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Optional hook for `Pools`. It should be marked on the `Hook` impl block of the `Pools` struct.
/// ```rust
/// #[hook]
/// impl Hook for MyPools {
/// ...
/// }
///
/// ```
#[proc_macro_attribute]
pub fn hook(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
