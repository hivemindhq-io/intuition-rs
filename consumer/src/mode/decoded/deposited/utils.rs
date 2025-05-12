use crate::{
    ConsumerError,
    EthMultiVault::Deposited,
    mode::{
        decoded::utils::{VaultUpdate, update_vault},
        types::DecodedConsumerContext,
        utils::{get_or_create_account, get_or_create_vault},
    },
    schemas::types::DecodedMessage,
    traits::{SharePriceEvent, VaultManager},
};
use alloy::primitives::Uint;
use async_trait::async_trait;
use models::{
    claim::Claim,
    deposit::Deposit,
    event::{Event, EventType},
    position::Position,
    predicate_object::PredicateObject,
    signal::Signal,
    term::TermType,
    traits::SimpleCrud,
    triple::Triple,
    types::U256Wrapper,
    vault::Vault,
};
use sqlx::{Postgres, Transaction};
use std::str::FromStr;
use tracing::info;

#[async_trait]
pub trait DepositedTrait: for<'a> SharePriceEvent {
    fn get_sender(&self) -> String;
    fn get_receiver(&self) -> String;
    fn get_vault_id(&self) -> Uint<256, 4>;
    fn get_is_triple(&self) -> bool;
    fn get_is_atom_wallet(&self) -> bool;
    fn get_sender_assets_after_total_fees(&self) -> U256Wrapper;
    fn get_receiver_total_shares_in_vault(&self) -> U256Wrapper;
    fn get_shares_for_receiver(&self) -> U256Wrapper;
    fn get_entry_fee(&self) -> U256Wrapper;
    fn get_curve_id(&self) -> U256Wrapper;
    /// This function creates a claim and predicate object
    async fn create_claim_and_predicate_object(
        &self,
        backend_schema: &str,
        triple: &Triple,
        position_id: &str,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<(), ConsumerError> {
        // Create claim
        info!("Creating claim");
        Claim::builder()
            .id(self.format_claim_id())
            .account_id(self.get_receiver())
            .position_id(position_id.to_string())
            .build()
            .upsert(backend_schema, tx.as_mut())
            .await?;

        info!("Claim created");
        // Update or create predicate object
        info!("Creating predicate object");
        let predicate_object_id = format!("{}-{}", triple.predicate_id, triple.object_id);
        match PredicateObject::find_by_id(predicate_object_id, backend_schema, tx.as_mut()).await? {
            Some(mut po) => {
                po.claim_count += 1;
                po.upsert(backend_schema, tx.as_mut()).await?;
            }
            None => {
                PredicateObject::builder()
                    .id(format!("{}-{}", triple.predicate_id, triple.object_id))
                    .predicate_id(triple.predicate_id.clone())
                    .object_id(triple.object_id.clone())
                    .claim_count(1)
                    .triple_count(1)
                    .build()
                    .upsert(backend_schema, tx.as_mut())
                    .await?;
            }
        };

        Ok(())
    }

    /// This function creates a deposit
    async fn create_deposit(
        &self,
        event: &DecodedMessage,
        backend_schema: &str,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<Deposit, ConsumerError> {
        Deposit::builder()
            .id(DecodedMessage::event_id(event))
            .sender_id(self.get_sender())
            .receiver_id(self.get_receiver())
            .receiver_total_shares_in_vault(self.get_receiver_total_shares_in_vault())
            .sender_assets_after_total_fees(self.get_sender_assets_after_total_fees())
            .shares_for_receiver(self.get_shares_for_receiver())
            .entry_fee(self.get_entry_fee())
            .term_id(self.get_vault_id())
            .curve_id(self.get_curve_id())
            .is_triple(self.get_is_triple())
            .is_atom_wallet(self.get_is_atom_wallet())
            .block_number(U256Wrapper::try_from(event.block_number)?)
            .block_timestamp(event.block_timestamp)
            .transaction_hash(event.transaction_hash.clone())
            .build()
            .upsert(backend_schema, tx.as_mut())
            .await
            .map_err(ConsumerError::ModelError)
    }

    /// This function creates an `Event` for the `Deposited` event
    async fn create_event(
        &self,
        decoded_consumer_context: &DecodedConsumerContext,
        event: &DecodedMessage,
        deposit_id: String,
    ) -> Result<Event, ConsumerError> {
        // Create the event
        let event = if self.get_is_triple() {
            Event::builder()
                .id(DecodedMessage::event_id(event))
                .event_type(EventType::Deposited)
                .deposit_id(deposit_id)
                .block_number(U256Wrapper::try_from(event.block_number)?)
                .block_timestamp(event.block_timestamp)
                .transaction_hash(event.transaction_hash.clone())
                .triple_id(self.get_vault_id())
                .build()
        } else {
            Event::builder()
                .id(DecodedMessage::event_id(event))
                .event_type(EventType::Deposited)
                .deposit_id(deposit_id)
                .block_number(U256Wrapper::try_from(event.block_number)?)
                .block_timestamp(event.block_timestamp)
                .transaction_hash(event.transaction_hash.clone())
                .atom_id(self.get_vault_id())
                .build()
        };

        event
            .upsert(
                &decoded_consumer_context.backend_schema,
                &decoded_consumer_context.pg_pool,
            )
            .await
            .map_err(ConsumerError::ModelError)
    }

    /// This function creates a new position
    async fn create_new_position(
        &self,
        position_id: String,
        backend_schema: &str,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<Position, ConsumerError> {
        // Check if a position already exists
        if let Some(position) =
            Position::find_by_id(position_id.clone(), backend_schema, tx.as_mut()).await?
        {
            info!("Position already exists, skipping.");
            return Ok(position);
        }

        Position::builder()
            .id(position_id.clone())
            .account_id(self.get_receiver())
            .term_id(self.get_vault_id())
            .curve_id(self.get_curve_id())
            .shares(self.get_receiver_total_shares_in_vault())
            .build()
            .upsert(backend_schema, tx.as_mut())
            .await
            .map_err(ConsumerError::ModelError)
    }

    /// This function creates a `Signal` for the `Deposited` event
    async fn create_signal(
        &self,
        decoded_consumer_context: &DecodedConsumerContext,
        event: &DecodedMessage,
        vault: &Vault,
    ) -> Result<(), ConsumerError> {
        if self.get_sender_assets_after_total_fees() > U256Wrapper::try_from(0)? {
            if !self.get_is_triple() {
                Signal::builder()
                    .id(DecodedMessage::event_id(event))
                    .account_id(self.get_sender())
                    .delta(self.get_sender_assets_after_total_fees())
                    .atom_id(vault.term_id.clone())
                    .deposit_id(DecodedMessage::event_id(event))
                    .block_number(U256Wrapper::try_from(event.block_number)?)
                    .block_timestamp(event.block_timestamp)
                    .transaction_hash(event.transaction_hash.clone())
                    .term_id(vault.term_id.clone())
                    .curve_id(U256Wrapper::from_str("1")?)
                    .build()
            } else {
                Signal::builder()
                    .id(DecodedMessage::event_id(event))
                    .account_id(self.get_sender())
                    .delta(self.get_sender_assets_after_total_fees())
                    .triple_id(vault.term_id.clone())
                    .deposit_id(DecodedMessage::event_id(event))
                    .block_number(U256Wrapper::try_from(event.block_number)?)
                    .block_timestamp(event.block_timestamp)
                    .transaction_hash(event.transaction_hash.clone())
                    .term_id(vault.term_id.clone())
                    .curve_id(U256Wrapper::from_str("1")?)
                    .build()
            }
            .upsert(
                &decoded_consumer_context.backend_schema,
                &decoded_consumer_context.pg_pool,
            )
            .await?;
        } else {
            info!("Sender assets after total fees is 0, nothing to do.");
        }
        Ok(())
    }

    /// This function formats the claim ID
    fn format_claim_id(&self) -> String {
        format!(
            "{}-1-{}",
            self.get_vault_id(),
            self.get_receiver().to_lowercase()
        )
    }

    /// This function formats the position ID
    fn format_position_id(&self) -> String {
        format!(
            "{}-1-{}",
            self.get_vault_id(),
            self.get_receiver().to_lowercase()
        )
    }

    /// This function handles an existing position
    async fn handle_existing_position(
        &self,
        backend_schema: &str,
        position: &mut Position,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<(), ConsumerError> {
        // Update or create position
        self.update_position(backend_schema, position, tx).await?;

        Ok(())
    }

    /// This function handles the creation of a new position
    async fn handle_new_position(
        &self,
        backend_schema: &str,
        position_id: &str,
        triple: Option<Triple>,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<(), ConsumerError> {
        self.create_new_position(position_id.to_string(), backend_schema, tx)
            .await?;
        // Create claim and predicate object
        if let Some(triple) = triple {
            self.create_claim_and_predicate_object(backend_schema, &triple, position_id, tx)
                .await?;
        }

        Ok(())
    }

    /// This function handles the position and claims
    async fn handle_position_and_claims(
        &self,
        decoded_consumer_context: &DecodedConsumerContext,
        tx: &mut Transaction<'_, Postgres>,
        current_share_price: U256Wrapper,
        total_shares: Uint<256, 4>,
    ) -> Result<(), ConsumerError> {
        let position_id = self.format_position_id();

        if let Some(mut position) = Position::find_by_id(
            position_id.clone(),
            &decoded_consumer_context.backend_schema,
            tx.as_mut(),
        )
        .await?
        {
            if self.get_receiver_total_shares_in_vault() > U256Wrapper::try_from(0)? {
                self.handle_existing_position(
                    &decoded_consumer_context.backend_schema,
                    &mut position,
                    tx,
                )
                .await?;
            }
        } else if self.get_receiver_total_shares_in_vault() > U256Wrapper::try_from(0)? {
            let triple = Triple::find_by_id(
                U256Wrapper::from(self.get_vault_id()),
                &decoded_consumer_context.backend_schema,
                tx.as_mut(),
            )
            .await?;

            self.handle_new_position(
                &decoded_consumer_context.backend_schema,
                &position_id,
                triple,
                tx,
            )
            .await?;
        } else {
            info!("No need to update position or claims.");
        }

        // Update vault values
        update_vault(
            VaultUpdate::Deposited {
                sender_assets_after_total_fees: self.get_sender_assets_after_total_fees(),
            },
            self.get_vault_id(),
            decoded_consumer_context,
            tx,
            current_share_price,
            total_shares,
        )
        .await?;

        Ok(())
    }

    /// This function initializes the accounts and vault
    async fn initialize_accounts_and_vault(
        &self,
        decoded_consumer_context: &DecodedConsumerContext,
        event: &DecodedMessage,
    ) -> Result<Vault, ConsumerError> {
        // Create accounts
        let _sender = get_or_create_account(self.get_sender(), decoded_consumer_context).await?;
        let _receiver =
            get_or_create_account(self.get_receiver(), decoded_consumer_context).await?;

        get_or_create_vault(
            self,
            Some(event.block_number),
            decoded_consumer_context,
            if self.get_is_triple() {
                TermType::Triple
            } else {
                TermType::Atom
            },
        )
        .await
    }

    /// This function updates the position
    async fn update_position(
        &self,
        backend_schema: &str,
        position: &mut Position,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<(), ConsumerError> {
        if position.shares != self.get_receiver_total_shares_in_vault() {
            position.shares = self.get_receiver_total_shares_in_vault();
            position
                .upsert(backend_schema, tx.as_mut())
                .await
                .map_err(ConsumerError::ModelError)?;
        }
        Ok(())
    }
}

impl SharePriceEvent for &Deposited {}

impl DepositedTrait for &Deposited {
    fn get_sender(&self) -> String {
        self.sender.to_string()
    }

    fn get_receiver(&self) -> String {
        self.receiver.to_string()
    }

    fn get_vault_id(&self) -> Uint<256, 4> {
        self.vaultId
    }

    fn get_is_triple(&self) -> bool {
        self.isTriple
    }

    fn get_is_atom_wallet(&self) -> bool {
        self.isAtomWallet
    }

    fn get_sender_assets_after_total_fees(&self) -> U256Wrapper {
        U256Wrapper::from(self.senderAssetsAfterTotalFees)
    }

    fn get_receiver_total_shares_in_vault(&self) -> U256Wrapper {
        self.receiverTotalSharesInVault.into()
    }

    fn get_shares_for_receiver(&self) -> U256Wrapper {
        self.sharesForReceiver.into()
    }

    fn get_entry_fee(&self) -> U256Wrapper {
        self.entryFee.into()
    }

    fn get_curve_id(&self) -> U256Wrapper {
        U256Wrapper::from_str("1").unwrap()
    }
}

/// This impl is used to convert the `Deposited` event into a `VaultManager`
#[async_trait]
impl VaultManager for &Deposited {
    fn term_id(&self) -> Result<U256Wrapper, ConsumerError> {
        Ok(self.vaultId.into())
    }

    fn curve_id(&self) -> Result<U256Wrapper, ConsumerError> {
        Ok(1.try_into()?)
    }

    async fn total_shares(
        &self,
        decoded_consumer_context: &DecodedConsumerContext,
        block_number: Option<i64>,
    ) -> Result<U256Wrapper, ConsumerError> {
        Ok(decoded_consumer_context
            .fetch_total_shares_in_vault(
                self.vaultId,
                block_number.ok_or(ConsumerError::BlockNumberNotFound)?,
            )
            .await?
            .into())
    }

    async fn current_share_price(
        &self,
        decoded_consumer_context: &DecodedConsumerContext,
        block_number: Option<i64>,
    ) -> Result<U256Wrapper, ConsumerError> {
        Ok(decoded_consumer_context
            .fetch_current_share_price(
                self.vaultId,
                block_number.ok_or(ConsumerError::BlockNumberNotFound)?,
            )
            .await?
            .into())
    }

    async fn position_count(
        &self,
        decoded_consumer_context: &DecodedConsumerContext,
    ) -> Result<i32, ConsumerError> {
        Ok(Position::count_by_vault_and_curve(
            self.vaultId.into(),
            "1".try_into()?,
            &decoded_consumer_context.pg_pool,
            &decoded_consumer_context.backend_schema,
        )
        .await? as i32)
    }
}
