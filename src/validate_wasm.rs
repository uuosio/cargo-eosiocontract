// Copyright 2018-2021 Parity Technologies (UK) Ltd.
// This file is part of cargo-contract.
//
// cargo-contract is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// cargo-contract is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with cargo-contract.  If not, see <http://www.gnu.org/licenses/>.

use anyhow::Result;
use colored::Colorize;
use impl_serde::serialize as serde_hex;
use parity_wasm::elements::Module;

/// Marker inserted by the ink! codegen for an error which can't
/// be checked at compile time.
const INK_ENFORCE_ERR: &str = "__ink_enforce_error_";

/// Errors which may occur when forwarding a call is not allowed.
///
/// We insert markers for these errors in the generated contract code.
/// This is necessary since we can't check these errors at compile time
/// of the contract.
/// `cargo-contract` checks the contract code for these error markers
/// when building a contract and fails if it finds markers.
///
/// # Important Note
///
/// This is a copy of the equivalent type in ink!, which currently resides
/// [here](https://github.com/paritytech/ink/blob/master/crates/lang/codegen/src/generator/cross_calling.rs).
/// This type must be compatible with the ink! version in order to decode
/// the error encoded in the marker.
#[derive(codec::Encode, codec::Decode)]
pub enum EnforcedErrors {
    /// The below error represents calling a `&mut self` message in a context that
    /// only allows for `&self` messages. This may happen under certain circumstances
    /// when ink! trait implementations are involved with long-hand calling notation.
    #[codec(index = 1)]
    CannotCallTraitMessage {
        /// The trait that defines the called message.
        trait_ident: String,
        /// The name of the called message.
        message_ident: String,
        /// The selector of the called message.
        message_selector: [u8; 4],
        /// Is `true` if the `self` receiver of the ink! message is `&mut self`.
        message_mut: bool,
    },
    /// The below error represents calling a constructor in a context that does
    /// not allow calling it. This may happen when the constructor defined in a
    /// trait is cross-called in another contract.
    /// This is not allowed since the contract to which a call is forwarded must
    /// already exist at the point when the call to it is made.
    #[codec(index = 2)]
    CannotCallTraitConstructor {
        /// The trait that defines the called constructor.
        trait_ident: String,
        /// The name of the called constructor.
        constructor_ident: String,
        /// The selector of the called constructor.
        constructor_selector: [u8; 4],
    },
}

/// Validates the import section in the Wasm.
///
/// The checks currently fall into two categories:
/// - Known bugs for which we want to recommend a solution.
/// - Markers inserted by the ink! codegen for errors which can't be checked at compile time.
pub fn validate_import_section(module: &Module) -> Result<()> {
    let imports = match module.import_section() {
        Some(section) => section.entries().iter(),
        None => {
            // the module does not contain any imports,
            // hence no further validation is necessary.
            return Ok(());
        }
    };
    let original_imports_len = imports.len();
    let mut errs = Vec::new();

    let filtered_imports = imports.filter(|section| {
        let field = section.field();
        if field.contains("panic") {
            errs.push(String::from(
                "An unexpected panic function import was found in the contract Wasm.\n\
                This typically goes back to a known bug in the Rust compiler:\n\
                https://github.com/rust-lang/rust/issues/78744\n\n\
                As a workaround try to insert `overflow-checks = false` into your `Cargo.toml`.\n\
                This will disable safe math operations, but unfortunately we are currently not \n\
                aware of a better workaround until the bug in the compiler is fixed.",
            ));
        } else if field.starts_with(INK_ENFORCE_ERR) {
            errs.push(parse_linker_error(field));
        }

        match check_import(field) {
            Ok(_) => true,
            Err(err) => {
                errs.push(err);
                false
            }
        }
    });

    if original_imports_len != filtered_imports.count() {
        anyhow::bail!(format!(
            "Validation of the Wasm failed.\n\n\n{}",
            errs.into_iter()
                .map(|err| format!("{} {}", "ERROR:".to_string().bold(), err))
                .collect::<Vec<String>>()
                .join("\n\n\n")
        ));
    }
    Ok(())
}

/// Returns `true` if the import is allowed.
fn check_import(field: &str) -> Result<(), String> {
    let allowed_functions = [
        "__ashrti3",
        "__lshlti3",
        "__lshrti3",
        "__ashlti3",
        "__divti3",
        "__udivti3",
        "__modti3",
        "__umodti3",
        "__multi3",
        "__addtf3",
        "__subtf3",
        "__multf3",
        "__divtf3",
        "__eqtf2",
        "__netf2",
        "__getf2",
        "__gttf2",
        "__lttf2",
        "__letf2",
        "__cmptf2",
        "__unordtf2",
        "__negtf2",
        "__floatsitf",
        "__floatunsitf",
        "__floatditf",
        "__floatunditf",
        "__floattidf",
        "__floatuntidf",
        "__floatsidf",
        "__extendsftf2",
        "__extenddftf2",
        "__fixtfti",
        "__fixtfdi",
        "__fixtfsi",
        "__fixunstfti",
        "__fixunstfdi",
        "__fixunstfsi",
        "__fixsfti",
        "__fixdfti",
        "__fixunssfti",
        "__fixunsdfti",
        "__trunctfdf2",
        "__trunctfsf2",
        "is_feature_active",
        "activate_feature",
        "get_resource_limits",
        "set_resource_limits",
        "set_proposed_producers",
        "get_blockchain_parameters_packed",
        "set_blockchain_parameters_packed",
        "is_privileged",
        "set_privileged",
        "get_active_producers",
        "db_idx64_store",
        "db_idx64_remove",
        "db_idx64_update",
        "db_idx64_find_primary",
        "db_idx64_find_secondary",
        "db_idx64_lowerbound",
        "db_idx64_upperbound",
        "db_idx64_end",
        "db_idx64_next",
        "db_idx64_previous",
        "db_idx128_store",
        "db_idx128_remove",
        "db_idx128_update",
        "db_idx128_find_primary",
        "db_idx128_find_secondary",
        "db_idx128_lowerbound",
        "db_idx128_upperbound",
        "db_idx128_end",
        "db_idx128_next",
        "db_idx128_previous",
        "db_idx256_store",
        "db_idx256_remove",
        "db_idx256_update",
        "db_idx256_find_primary",
        "db_idx256_find_secondary",
        "db_idx256_lowerbound",
        "db_idx256_upperbound",
        "db_idx256_end",
        "db_idx256_next",
        "db_idx256_previous",
        "db_idx_double_store",
        "db_idx_double_remove",
        "db_idx_double_update",
        "db_idx_double_find_primary",
        "db_idx_double_find_secondary",
        "db_idx_double_lowerbound",
        "db_idx_double_upperbound",
        "db_idx_double_end",
        "db_idx_double_next",
        "db_idx_double_previous",
        "db_idx_long_double_store",
        "db_idx_long_double_remove",
        "db_idx_long_double_update",
        "db_idx_long_double_find_primary",
        "db_idx_long_double_find_secondary",
        "db_idx_long_double_lowerbound",
        "db_idx_long_double_upperbound",
        "db_idx_long_double_end",
        "db_idx_long_double_next",
        "db_idx_long_double_previous",
        "db_store_i64",
        "db_update_i64",
        "db_remove_i64",
        "db_get_i64",
        "db_next_i64",
        "db_previous_i64",
        "db_find_i64",
        "db_lowerbound_i64",
        "db_upperbound_i64",
        "db_end_i64",
        "assert_recover_key",
        "recover_key",
        "assert_sha256",
        "assert_sha1",
        "assert_sha512",
        "assert_ripemd160",
        "sha1",
        "sha256",
        "sha512",
        "ripemd160",
        "check_transaction_authorization",
        "check_permission_authorization",
        "get_permission_last_used",
        "get_account_creation_time",
        "current_time",
        "publication_time",
        "abort",
        "eosio_assert",
        "eosio_assert_message",
        "eosio_assert_code",
        "eosio_exit",
        "read_action_data",
        "action_data_size",
        "current_receiver",
        "require_recipient",
        "require_auth",
        "require_auth2",
        "has_auth",
        "is_account",
        "prints",
        "prints_l",
        "printi",
        "printui",
        "printi128",
        "printui128",
        "printsf",
        "printdf",
        "printqf",
        "printn",
        "printhex",
        "read_transaction",
        "transaction_size",
        "expiration",
        "tapos_block_prefix",
        "tapos_block_num",
        "get_action",
        "send_inline",
        "send_context_free_inline",
        "send_deferred",
        "cancel_deferred",
        "get_context_free_data",
        "memcpy",
        "memmove",
        "memcmp",
        "memset",
    ];
    if allowed_functions
        .iter()
        .any(|function| field.eq(*function))
    {
        Ok(())
    } else {
        let msg = format!(
            "An unexpected import function was found in the contract Wasm: {}.\n",
            field
        );
        Err(msg)
    }
}

/// Extracts the ink! linker error marker from the `field`, parses it, and
/// returns a human readable error message for it.
fn parse_linker_error(field: &str) -> String {
    let encoded = field
        .strip_prefix(INK_ENFORCE_ERR)
        .expect("error marker must exist as prefix");
    let hex = serde_hex::from_hex(encoded).expect("decoding hex failed");
    let decoded = <EnforcedErrors as codec::Decode>::decode(&mut &hex[..]).expect(
        "The `EnforcedError` object could not be decoded. The probable\
        cause is a mismatch between the ink! definition of the type and the\
        local `cargo-contract` definition.",
    );

    match decoded {
        EnforcedErrors::CannotCallTraitMessage {
            trait_ident,
            message_ident,
            message_selector,
            message_mut,
        } => {
            let receiver = match message_mut {
                true => "&mut self",
                false => "&self",
            };
            format!(
                "An error was found while compiling the contract:\n\
                The ink! message `{}::{}` with the selector `{}` contains an invalid trait call.\n\n\
                Please check if the receiver of the function to call is consistent\n\
                with the scope in which it is called. The receiver is `{}`.",
                trait_ident,
                message_ident,
                serde_hex::to_hex(&codec::Encode::encode(&message_selector), false),
                receiver
            )
        }
        EnforcedErrors::CannotCallTraitConstructor {
            trait_ident,
            constructor_ident,
            constructor_selector,
        } => {
            format!(
                "An error was found while compiling the contract:\n\
                The ink! constructor `{}::{}` with the selector `{}` contains an invalid trait call.\n\
                Constructor never need to be forwarded, please check if this is the case.",
                trait_ident,
                constructor_ident,
                serde_hex::to_hex(&codec::Encode::encode(&constructor_selector), false)
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::validate_import_section;
    use parity_wasm::elements::Module;

    fn create_module(contract: &str) -> Module {
        let wasm = wabt::wat2wasm(contract).expect("invalid wabt");
        parity_wasm::deserialize_buffer(&wasm).expect("deserializing must work")
    }

    #[test]
    fn must_catch_panic_import() {
        // given
        let contract = r#"
            (module
                (type (;0;) (func (param i32 i32 i32)))
                (import "env" "_ZN4core9panicking5panic17h00e3acdd8048cb7cE" (func (;5;) (type 0)))
                (func (;5;) (type 0))
            )"#;
        let module = create_module(contract);

        // when
        let res = validate_import_section(&module);

        // then
        assert!(res.is_err());
        assert!(res
            .unwrap_err()
            .to_string()
            .contains("An unexpected panic function import was found in the contract Wasm."));
    }

    #[test]
    fn must_catch_ink_enforce_error_marker_message() {
        // given
        let contract = r#"
            (module
                (type (;0;) (func))
                (import "env" "__ink_enforce_error_0x0110466c697010666c6970aa97cade01" (func $__ink_enforce_error_0x0110466c697010666c6970aa97cade01 (type 0)))
            )"#;
        let wasm = wabt::wat2wasm(contract).expect("invalid wabt");
        let module = parity_wasm::deserialize_buffer(&wasm).expect("deserializing must work");

        // when
        let res = validate_import_section(&module);

        // then
        assert!(res.is_err());
        let err = res.unwrap_err().to_string();
        assert!(err.contains(
            "The ink! message `Flip::flip` with the selector `0xaa97cade` contains an invalid trait call."
        ));
        assert!(err.contains("The receiver is `&mut self`.",));
    }

    #[test]
    fn must_catch_ink_enforce_error_marker_constructor() {
        // given
        let contract = r#"
            (module
                (type (;0;) (func))
                (import "env" "__ink_enforce_error_0x0210466c69700c6e657740d75d74" (func $__ink_enforce_error_0x0210466c69700c6e657740d75d74 (type 0)))
            )"#;
        let module = create_module(contract);

        // when
        let res = validate_import_section(&module);

        // then
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains(
            "The ink! constructor `Flip::new` with the selector `0x40d75d74` contains an invalid trait call."
        ));
    }

    #[test]
    fn must_catch_invalid_import() {
        // given
        let contract = r#"
            (module
                (type (;0;) (func (param i32 i32 i32)))
                (import "env" "some_fn" (func (;5;) (type 0)))
                (func (;5;) (type 0))
            )"#;
        let module = create_module(contract);

        // when
        let res = validate_import_section(&module);

        // then
        assert!(res.is_err());
        assert!(res
            .unwrap_err()
            .to_string()
            .contains("An unexpected import function was found in the contract Wasm: some_fn."));
    }

    #[test]
    fn must_validate_successfully() {
        // given
        let contract = r#"
            (module
                (type (;0;) (func (param i32 i32 i32)))
                (import "env" "seal_foo" (func (;5;) (type 0)))
                (import "env" "memory" (func (;5;) (type 0)))
                (func (;5;) (type 0))
            )"#;
        let module = create_module(contract);

        // when
        let res = validate_import_section(&module);

        // then
        assert!(res.is_ok());
    }

    #[test]
    fn must_validate_successfully_if_no_import_section_found() {
        // given
        let contract = r#"(module)"#;
        let module = create_module(contract);

        // when
        let res = validate_import_section(&module);

        // then
        assert!(res.is_ok());
    }
}
