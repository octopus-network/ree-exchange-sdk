use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use std::collections::BTreeMap;
use syn::{Attribute, Ident, ItemMod, parse_macro_input, parse_quote, visit_mut::VisitMut};

#[derive(Clone)]
struct CanisterVisitor {
    actions: BTreeMap<String, (String, bool)>,
    pools: Option<Ident>,
    hook_present: bool,
    upgrade_declared: bool,
    storages: BTreeMap<u8, (proc_macro2::TokenStream, proc_macro2::TokenStream)>,
}

mod keywords {
    syn::custom_keyword!(exchange);
    syn::custom_keyword!(upgrade);
    syn::custom_keyword!(pools);
    syn::custom_keyword!(hook);
    syn::custom_keyword!(storage);
    syn::custom_keyword!(action);
    syn::custom_keyword!(memory);
    syn::custom_keyword!(name);
}

struct StorageDeclAttr {
    memory_id: u8,
}

impl syn::parse::Parse for StorageDeclAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        input.parse::<syn::Token![#]>()?;
        let content;
        syn::bracketed!(content in input);
        content.parse::<keywords::storage>()?;
        let inside;
        syn::parenthesized!(inside in content);
        let lookahead = inside.lookahead1();
        if lookahead.peek(keywords::memory) {
            let _ = inside.parse::<keywords::memory>()?;
            let _ = inside.parse::<syn::Token![=]>()?;
            let lit: syn::LitInt = inside.parse()?;
            let memory_id = lit.base10_parse::<u8>()?;
            if memory_id >= 100 {
                return Err(syn::Error::new_spanned(
                    lit,
                    "Memory id must be between 0 and 99",
                ));
            }
            Ok(Self { memory_id })
        } else {
            let lit: syn::LitInt = inside.parse()?;
            let memory_id = lit.base10_parse::<u8>()?;
            if memory_id >= 100 {
                return Err(syn::Error::new_spanned(
                    lit,
                    "Memory id must be between 0 and 99",
                ));
            }
            Ok(Self { memory_id })
        }
    }
}

enum ActionDeclAttr {
    Named { value: syn::LitStr },
    Unnamed,
}

impl syn::parse::Parse for ActionDeclAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        input.parse::<syn::Token![#]>()?;
        let content;
        syn::bracketed!(content in input);
        content.parse::<keywords::action>()?;
        if content.is_empty() {
            return Ok(Self::Unnamed);
        }
        let inside;
        syn::parenthesized!(inside in content);
        let lookahead = inside.lookahead1();
        if lookahead.peek(keywords::name) {
            let _ = inside.parse::<keywords::name>()?;
            let _ = inside.parse::<syn::Token![=]>()?;
            Ok(Self::Named {
                value: inside.parse()?,
            })
        } else if lookahead.peek(syn::LitStr) {
            Ok(Self::Named {
                value: inside.parse()?,
            })
        } else {
            Err(lookahead.error())
        }
    }
}

impl CanisterVisitor {
    fn new() -> Self {
        CanisterVisitor {
            actions: BTreeMap::new(),
            pools: None,
            hook_present: false,
            upgrade_declared: false,
            storages: BTreeMap::new(),
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

    fn resolve_action(&mut self, attr: &Attribute, func: &syn::ItemFn) {
        let is_action = attr.path().is_ident("action");
        if !is_action {
            return;
        }
        let tokens = attr.to_token_stream();
        let action_decl =
            syn::parse2::<ActionDeclAttr>(tokens).expect("Failed to parse action attribute");
        match action_decl {
            ActionDeclAttr::Unnamed => {
                self.actions.insert(
                    func.sig.ident.to_string(),
                    (func.sig.ident.to_string(), func.sig.asyncness.is_some()),
                );
            }
            ActionDeclAttr::Named { value, .. } => {
                let action = value.value();
                self.actions.insert(
                    action,
                    (func.sig.ident.to_string(), func.sig.asyncness.is_some()),
                );
            }
        }
    }

    fn resolve_storage(&mut self, attr: &Attribute, ty: &syn::ItemType) {
        let is_storage = attr.path().is_ident("storage");
        if !is_storage {
            return;
        }
        let tokens = attr.to_token_stream();
        let storage_decl =
            syn::parse2::<StorageDeclAttr>(tokens).expect("Failed to parse storage attribute");
        let id = storage_decl.memory_id;
        let storage_name = to_upper_snake_case(&ty.ident.to_string());
        let storage_name = format_ident!("__{}", storage_name);
        let storage_ty = format_ident!("{}", ty.ident);
        let ic_ty = quote! { <#storage_ty as ::ree_exchange_sdk::store::StorageType>::Type };
        let decl = quote! {
            static #storage_name: ::core::cell::RefCell<#ic_ty> = ::core::cell::RefCell::new(
                <#storage_ty as ::ree_exchange_sdk::store::StorageType>::init(
                    __MEMORY_MANAGER.with(|m| m.borrow().get(::ic_stable_structures::memory_manager::MemoryId::new(#id))),
                )
            );
        };
        let access = quote! {
            impl __CustomStorageAccess<#storage_ty> for #storage_ty {
                fn with<F, R>(f: F) -> R
                where
                    F: FnOnce(&#ic_ty) -> R,
                {
                    #storage_name.with(|s| {
                        let s = s.borrow();
                        let r = <::std::cell::Ref<'_, #ic_ty> as ::std::ops::Deref>::deref(&s);
                        f(r)
                    })
                }

                fn with_mut<F, R>(f: F) -> R
                where
                    F: FnOnce(&mut #ic_ty) -> R,
                {
                    #storage_name.with(|s| {
                        let mut s = s.borrow_mut();
                        let r = <::std::cell::RefMut<'_, #ic_ty> as ::std::ops::DerefMut>::deref_mut(&mut s);
                        f(r)
                    })
                }
            }
        };
        if let Some(_) = self.storages.insert(id, (decl, access)) {
            panic!("Memory id {} is already used", id);
        }
    }
}

fn to_upper_snake_case(s: &str) -> String {
    let mut snake_case = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i != 0 {
                snake_case.push('_');
            }
            snake_case.push(ch);
        } else {
            snake_case.push(ch.to_ascii_uppercase());
        }
    }
    snake_case
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
        if let Some(_attr) = item.attrs.iter().find(|a| a.path().is_ident("upgrade")) {
            self.upgrade_declared = true;
        }
        syn::visit_mut::visit_item_impl_mut(self, item);
    }

    fn visit_item_type_mut(&mut self, item: &mut syn::ItemType) {
        for attr in item.attrs.iter() {
            self.resolve_storage(&attr, item);
        }
        syn::visit_mut::visit_item_type_mut(self, item);
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
    let (storage_decl, storage_access): (
        Vec<proc_macro2::TokenStream>,
        Vec<proc_macro2::TokenStream>,
    ) = visitor.storages.into_values().unzip();
    let pools = visitor.pools.clone().unwrap();
    if let Some((_, ref mut items)) = input_mod.content {
        let branch = visitor
            .actions
            .iter()
            .map(|(action, (func, is_async))| {
                let call = format_ident!("{}", func);
                if *is_async {
                    quote! { #action => #call(&psbt, args).await, }
                } else {
                    quote! { #action => #call(&psbt, args), }
                }
            })
            .collect::<Vec<_>>();

        if !visitor.hook_present {
            items.push(parse_quote! {
                impl ::ree_exchange_sdk::Hook for #pools {}
            });
        }

        items.push(parse_quote! {
            impl ::ree_exchange_sdk::PoolStorageAccess<#pools> for #pools {
                fn block_state() -> ::std::option::Option<<#pools as ::ree_exchange_sdk::Pools>::BlockState> {
                    self::__GLOBAL_STATE.with_borrow(|p| p.last_key_value().map(|(k, v)| v.inner))
                }

                fn commit(height: u32, block_state: <#pools as ::ree_exchange_sdk::Pools>::BlockState) -> ::std::result::Result<(), ::std::string::String> {
                    self::__GLOBAL_STATE.with_borrow_mut(|p| {
                        if p.contains_key(&height) {
                            ::std::result::Result::Err(format!("Block state for height {} already exists", height))
                        } else {
                            p.insert(height, ::ree_exchange_sdk::GlobalStateWrapper { inner: block_state });
                            ::std::result::Result::Ok(())
                        }
                    })
                }

                fn get(address: &::std::string::String) -> ::std::option::Option<::ree_exchange_sdk::Pool<<#pools as ::ree_exchange_sdk::Pools>::PoolState>> {
                    self::__CURRENT_POOLS.with_borrow(|p| p.get(address))
                }

                fn insert(pool: ::ree_exchange_sdk::Pool<<#pools as ::ree_exchange_sdk::Pools>::PoolState>) {
                    self::__CURRENT_POOLS.with_borrow_mut(|p| {
                        p.insert(pool.metadata().address.clone(), pool);
                    });
                }

                fn remove(address: &::std::string::String) -> ::std::option::Option<::ree_exchange_sdk::Pool<<#pools as ::ree_exchange_sdk::Pools>::PoolState>> {
                    self::__CURRENT_POOLS.with_borrow_mut(|p| {
                        p.remove(address)
                    })
                }

                fn iter() -> ::ree_exchange_sdk::iter::PoolIterator<#pools> {
                    let memory = __MEMORY_MANAGER.with(|m| m.borrow().get(::ic_stable_structures::memory_manager::MemoryId::new(
                        <#pools as ::ree_exchange_sdk::Pools>::POOL_STATE_MEMORY
                    )));
                    ::ree_exchange_sdk::iterator::<#pools>(memory)
                }
            }
        });

        items.push(parse_quote! {
            #[::ic_cdk::update]
            pub async fn execute_tx(args: ::ree_exchange_sdk::types::exchange_interfaces::ExecuteTxArgs) -> ::core::result::Result<String, String> {
                ::ree_exchange_sdk::ensure_access::<#pools>()?;
                let mut psbt = args.psbt()?;
                let args = <::ree_exchange_sdk::ActionArgs as ::std::convert::From<_>>::from(args);
                let pool_address = args.intention.pool_address.clone();
                let _guard = self::__ExecuteTxGuard::new(pool_address.clone())
                    .ok_or(format!("Pool {} is being executed", pool_address))?;
                let txid = args.txid.clone();
                let inputs = args.intention.pool_outpoints()
                    .map_err(|e| format!("Failed to deserialize input outpoints: {}", e))?;
                let action = args.intention.action.clone();
                let result: ::ree_exchange_sdk::ActionResult::<<#pools as ::ree_exchange_sdk::Pools>::PoolState> = match action.as_str() {
                    #(#branch)*
                    _ => ::ree_exchange_sdk::ActionResult::<<#pools as ::ree_exchange_sdk::Pools>::PoolState>::Err(format!("Unknown action: {}", action)),
                };
                match result {
                    ::ree_exchange_sdk::ActionResult::<<#pools as ::ree_exchange_sdk::Pools>::PoolState>::Ok(r) => {
                        let mut pool = self::__CURRENT_POOLS.with_borrow(|pools| {
                            pools.get(&pool_address).clone()
                        }).ok_or(format!("Pool {} not found", pool_address))?;
                        ::ree_exchange_sdk::schnorr::sign_p2tr_inputs(
                            &mut psbt,
                            &inputs,
                            <#pools as ::ree_exchange_sdk::Pools>::network(),
                            pool.metadata().key_derivation_path.clone(),
                        ).await?;
                        pool.states_mut().push(r);
                        self::__CURRENT_POOLS.with_borrow_mut(|pools| {
                            pools.insert(pool_address.clone(), pool);
                        });
                        self::__TX_RECORDS.with_borrow_mut(|unconfirmed| {
                            let mut record = unconfirmed.get(&txid).unwrap_or(::ree_exchange_sdk::types::TxRecord {
                                txid: txid.clone(),
                                pools: vec![pool_address.clone()],
                            });
                            if !record.pools.contains(&pool_address) {
                                record.pools.push(pool_address.clone());
                            }
                            unconfirmed.insert(txid, record);
                        });
                        ::core::result::Result::<String, String>::Ok(psbt.serialize_hex())
                    }
                    ::ree_exchange_sdk::ActionResult::<<#pools as ::ree_exchange_sdk::Pools>::PoolState>::Err(e) => {
                        ::core::result::Result::<String, String>::Err(e)
                    }
                }
            }
        });

        items.push(parse_quote! {
            #[::ic_cdk::query]
            pub fn get_pool_list() -> ::ree_exchange_sdk::types::exchange_interfaces::GetPoolListResponse {
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
                args: ::ree_exchange_sdk::types::exchange_interfaces::GetPoolInfoArgs,
            ) -> ::ree_exchange_sdk::types::exchange_interfaces::GetPoolInfoResponse {
                self::__CURRENT_POOLS.with_borrow(|pools| {
                    pools.get(&args.pool_address).map(|p| p.get_pool_info())
                })
            }
        });

        items.push(parse_quote! {
            #[::ic_cdk::update]
            pub fn rollback_tx(
                args: ::ree_exchange_sdk::types::exchange_interfaces::RollbackTxArgs,
            ) -> ::ree_exchange_sdk::types::exchange_interfaces::RollbackTxResponse {
                ::ree_exchange_sdk::ensure_access::<#pools>()?;
                self::__TX_RECORDS.with_borrow_mut(|transactions| {
                    self::__CURRENT_POOLS.with_borrow_mut(|pools| {
                        ::ree_exchange_sdk::states::reject_tx::<#pools>(transactions, pools, args)
                    })
                })
            }
        });

        items.push(parse_quote! {
            #[::ic_cdk::update]
            pub fn new_block(
                args: ::ree_exchange_sdk::types::exchange_interfaces::NewBlockArgs,
            ) -> ::ree_exchange_sdk::types::exchange_interfaces::NewBlockResponse {
                ::ree_exchange_sdk::ensure_access::<#pools>()?;
                let block = self::__TX_RECORDS.with_borrow_mut(|unconfirmed| {
                    self::__BLOCKS.with_borrow_mut(|blocks| {
                        self::__GLOBAL_STATE.with_borrow_mut(|state| {
                            ::ree_exchange_sdk::states::confirm_txs::<#pools>(
                                state,
                                blocks,
                                unconfirmed,
                                args,
                            )
                        })
                    })
                })?;
                if let Some(block) = block {
                    self::__CURRENT_POOLS.with_borrow_mut(|pools| {
                        self::__BLOCKS.with_borrow_mut(|blocks| {
                            self::__GLOBAL_STATE.with_borrow_mut(|state| {
                                ::ree_exchange_sdk::states::accept_block::<#pools>(
                                    state,
                                    blocks,
                                    pools,
                                    block.clone(),
                                )
                            })
                        })
                    })?;
                    <#pools as ::ree_exchange_sdk::Hook>::on_block_confirmed(block);
                }
                Ok(())
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
                        ::ree_exchange_sdk::Block,
                        ::ic_stable_structures::memory_manager::VirtualMemory<::ic_stable_structures::DefaultMemoryImpl>
                    >
                > = ::core::cell::RefCell::new(
                    ::ic_stable_structures::StableBTreeMap::init(
                        __MEMORY_MANAGER.with(|m| m.borrow().get(::ic_stable_structures::memory_manager::MemoryId::new(
                            100
                        ))),
                    )
                );
                static __TX_RECORDS: ::core::cell::RefCell<
                    ::ic_stable_structures::StableBTreeMap<
                        ::ree_exchange_sdk::types::Txid,
                        ::ree_exchange_sdk::types::TxRecord,
                        ::ic_stable_structures::memory_manager::VirtualMemory<::ic_stable_structures::DefaultMemoryImpl>
                    >
                > = ::core::cell::RefCell::new(
                    ::ic_stable_structures::StableBTreeMap::init(
                        __MEMORY_MANAGER.with(|m| m.borrow().get(::ic_stable_structures::memory_manager::MemoryId::new(
                            101
                        ))),
                    )
                );
                static __CURRENT_POOLS: ::core::cell::RefCell<
                    ::ic_stable_structures::StableBTreeMap<
                        ::std::string::String,
                        ::ree_exchange_sdk::Pool<
                            <#pools as ::ree_exchange_sdk::Pools>::PoolState
                        >,
                        ::ic_stable_structures::memory_manager::VirtualMemory<::ic_stable_structures::DefaultMemoryImpl>
                    >
                > = ::core::cell::RefCell::new(
                    ::ic_stable_structures::StableBTreeMap::init(
                        __MEMORY_MANAGER.with(|m| m.borrow().get(::ic_stable_structures::memory_manager::MemoryId::new(
                            <#pools as ::ree_exchange_sdk::Pools>::POOL_STATE_MEMORY
                        ))),
                    )
                );
                static __GLOBAL_STATE: ::core::cell::RefCell<
                    ::ic_stable_structures::StableBTreeMap<
                        u32,
                        ::ree_exchange_sdk::GlobalStateWrapper<<#pools as ::ree_exchange_sdk::Pools>::BlockState>,
                        ::ic_stable_structures::memory_manager::VirtualMemory<::ic_stable_structures::DefaultMemoryImpl>
                    >
                > = ::core::cell::RefCell::new(
                    ::ic_stable_structures::StableBTreeMap::init(
                        __MEMORY_MANAGER.with(|m| m.borrow().get(::ic_stable_structures::memory_manager::MemoryId::new(
                            <#pools as ::ree_exchange_sdk::Pools>::BLOCK_STATE_MEMORY
                        ))),
                    )
                );
                #(#storage_decl)*
            }
        });

        if visitor.upgrade_declared {
            items.push(parse_quote! {
                impl #pools {
                    pub fn upgrade() {
                        let pool_id = <#pools as ::ree_exchange_sdk::Upgrade<#pools>>::POOL_STATE_MEMORY;
                        if pool_id >= 100 {
                            panic!("Memory id for pool state upgrade must be between 0 and 99");
                        }
                        let block_id = <#pools as ::ree_exchange_sdk::Upgrade<#pools>>::BLOCK_STATE_MEMORY;
                        if block_id >= 100 {
                            panic!("Memory id for block state upgrade must be between 0 and 99");
                        }
                        let memory_id = ::ic_stable_structures::memory_manager::MemoryId::new(pool_id);
                        let memory = __MEMORY_MANAGER.with(|m| m.borrow().get(memory_id));
                        let mut pool_storage = ::ic_stable_structures::StableBTreeMap::<
                            ::std::string::String,
                            ::ree_exchange_sdk::Pool<<#pools as ::ree_exchange_sdk::Upgrade<#pools>>::PoolState>,
                            ::ic_stable_structures::memory_manager::VirtualMemory<::ic_stable_structures::DefaultMemoryImpl>,
                        >::init(memory);
                        self::__CURRENT_POOLS.with_borrow_mut(|pools| {
                            for entry in pool_storage.iter() {
                                let old_pool = entry.value();
                                let states = old_pool.states()
                                    .iter()
                                    .map(|s| <<#pools as ::ree_exchange_sdk::Upgrade<#pools>>::PoolState as ::std::clone::Clone>::clone(s))
                                    .map(|s| <<#pools as ::ree_exchange_sdk::Upgrade<#pools>>::PoolState as ::std::convert::Into<<#pools as ::ree_exchange_sdk::Pools>::PoolState>>::into(s))
                                    .collect::<Vec<<#pools as ::ree_exchange_sdk::Pools>::PoolState>>();
                                let mut new_pool = ::ree_exchange_sdk::Pool::new(
                                    old_pool.metadata().clone(),
                                );
                                new_pool.states_mut().extend(states);
                                pools.insert(entry.key().clone(), new_pool);
                            }
                        });

                        let memory_id = ::ic_stable_structures::memory_manager::MemoryId::new(block_id);
                        let memory = __MEMORY_MANAGER.with(|m| m.borrow().get(memory_id));
                        let mut block_storage = ::ic_stable_structures::StableBTreeMap::<
                            u32,
                            ::ree_exchange_sdk::GlobalStateWrapper<<#pools as ::ree_exchange_sdk::Upgrade<#pools>>::BlockState>,
                            ::ic_stable_structures::memory_manager::VirtualMemory<::ic_stable_structures::DefaultMemoryImpl>,
                        >::init(memory);
                        self::__GLOBAL_STATE.with_borrow_mut(|blocks| {
                            for entry in block_storage.iter() {
                                let old_block = entry.value().inner;
                                let height = *entry.key();
                                let new_block = <<#pools as ::ree_exchange_sdk::Upgrade<#pools>>::BlockState as ::std::convert::Into<<#pools as ::ree_exchange_sdk::Pools>::BlockState>>::into(old_block);
                                blocks.insert(height, ::ree_exchange_sdk::GlobalStateWrapper { inner: new_block });
                            }
                        });
                        pool_storage.clear_new();
                        block_storage.clear_new();
                    }
                }
            });
        }

        items.push(parse_quote! {
            pub trait __CustomStorageAccess<S: ::ree_exchange_sdk::store::StorageType> {
                fn with<F, R>(f: F) -> R
                where
                    F: FnOnce(&S::Type) -> R;
                fn with_mut<F, R>(f: F) -> R
                where
                    F: FnOnce(&mut S::Type) -> R;
            }
        });
        for access in storage_access {
            items.push(parse_quote! {
                #access
            });
        }
    }
    quote! {
        #input_mod
    }
    .into()
}

/// Action entrypoint. The macro could be
/// `#[action(name = "my_action")]` or `#[action("my_action")]` or `#[action]`.
/// The functions shall have signature `fn(&bitcoin::Psbt, ActionArgs) -> ActionResult<Pools::PoolState>`
#[proc_macro_attribute]
pub fn action(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Pools definition
#[proc_macro_attribute]
pub fn pools(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Storage definition
/// ```rust
/// #[storage(memory = 3)]
/// pub type MyStorage = ree_exchange_sdk::store::StableBTreeMap<String, String>;
/// ```
#[proc_macro_attribute]
pub fn storage(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Optional hook for `Pools`. It should be marked on the `Hook` impl block of the `Pools` struct.
/// ```rust
/// #[hook]
/// impl Hook for MyPools {
///     fn on_block_confirmed(block: Block) {
///         for tx in block.txs {
///             // for each pool affected by this tx
///             for addr in tx.pools {
///                 let pool = DummyPools::get(&addr).unwrap();
///                 // load the state and do something with it
///                 let _state = pool.get(tx.txid).unwrap();
///             }
///         }
///         // update block state
///         MyPools::commit(block.block_height, block.block_height);
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn hook(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Upgrade attribute for pool state migration.
#[proc_macro_attribute]
pub fn upgrade(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
