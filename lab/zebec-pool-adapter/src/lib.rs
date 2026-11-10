#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, Address, Bytes, BytesN, Env, IntoVal, Symbol, Val,
};

/// Testnet adapter: one Soroban invocation sequences a Zebec stream leg and a
/// privacy-pool `transact`, so the combined call is what the network budgets.
#[contract]
pub struct ZebecPoolAdapter;

#[contracttype]
#[derive(Clone)]
pub struct KytPassageAuthorization {
    pub expiration_ledger: u32,
    pub signature: BytesN<64>,
}

/// Stream leg of `withdraw_to_stream`, bundled so the entrypoint stays within
/// Soroban's 10-argument limit.
#[contracttype]
#[derive(Clone)]
pub struct StreamOpen {
    pub zebec: Address,
    pub stream_id: BytesN<32>,
    pub receiver: Address,
    pub token: Address,
    pub params: CreateStreamParams,
}

/// Field names match the Zebec streaming contract `CreateStreamParams`.
#[contracttype]
#[derive(Clone)]
pub struct CreateStreamParams {
    pub accrual_frequency: i64,
    pub amount: i64,
    pub auto_topup: bool,
    pub automatic_withdrawal: bool,
    pub cancelable_by_recipient: bool,
    pub cancelable_by_sender: bool,
    pub cliff_percentage: i64,
    pub duration: i64,
    pub initial_buffer_amount: i64,
    pub pausable: bool,
    pub receiver_id: soroban_sdk::String,
    pub sender_id: soroban_sdk::String,
    pub start_now: bool,
    pub start_time: i64,
    pub stream_name: soroban_sdk::String,
    pub transferable_by_recipient: bool,
    pub transferable_by_sender: bool,
}

#[contractimpl]
impl ZebecPoolAdapter {
    /// Withdraw the vested balance of `stream_id` to `caller`, then deposit it
    /// into the privacy pool. `caller` must be the stream receiver.
    pub fn deposit_from_stream(
        env: Env,
        caller: Address,
        zebec: Address,
        stream_id: BytesN<32>,
        pool: Address,
        nonce: u64,
        proof_bytes: Bytes,
        pub_signals_bytes: Bytes,
        ciphertext_bytes: Bytes,
        kyt_authorization: Option<KytPassageAuthorization>,
    ) {
        caller.require_auth();
        env.invoke_contract::<()>(
            &zebec,
            &Symbol::new(&env, "withdraw_stream"),
            (stream_id, caller.clone()).into_val(&env),
        );
        transact(
            &env,
            &pool,
            &caller,
            nonce,
            &proof_bytes,
            &pub_signals_bytes,
            &ciphertext_bytes,
            kyt_authorization,
        );
    }

    /// Withdraw publicly from the pool to `caller`, then lock `params.amount`
    /// into a new Zebec stream sent by `caller`.
    pub fn withdraw_to_stream(
        env: Env,
        caller: Address,
        pool: Address,
        nonce: u64,
        proof_bytes: Bytes,
        pub_signals_bytes: Bytes,
        ciphertext_bytes: Bytes,
        kyt_authorization: Option<KytPassageAuthorization>,
        stream: StreamOpen,
    ) {
        caller.require_auth();
        transact(
            &env,
            &pool,
            &caller,
            nonce,
            &proof_bytes,
            &pub_signals_bytes,
            &ciphertext_bytes,
            kyt_authorization,
        );
        env.invoke_contract::<()>(
            &stream.zebec,
            &Symbol::new(&env, "create_stream"),
            (
                stream.stream_id,
                caller,
                stream.receiver,
                stream.token,
                stream.params,
            )
                .into_val(&env),
        );
    }
}

fn transact(
    env: &Env,
    pool: &Address,
    caller: &Address,
    nonce: u64,
    proof_bytes: &Bytes,
    pub_signals_bytes: &Bytes,
    ciphertext_bytes: &Bytes,
    kyt_authorization: Option<KytPassageAuthorization>,
) {
    let kyt: Val = match kyt_authorization {
        Some(authorization) => authorization.into_val(env),
        None => ().into_val(env),
    };
    env.invoke_contract::<()>(
        pool,
        &Symbol::new(env, "transact"),
        (
            caller.clone(),
            nonce,
            proof_bytes.clone(),
            pub_signals_bytes.clone(),
            ciphertext_bytes.clone(),
            kyt,
        )
            .into_val(env),
    );
}
