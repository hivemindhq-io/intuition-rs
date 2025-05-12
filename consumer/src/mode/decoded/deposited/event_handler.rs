use crate::{
    ConsumerError, mode::types::DecodedConsumerContext, schemas::types::DecodedMessage,
    traits::EventHandler,
};
use models::{
    event::{Event, EventType},
    traits::SimpleCrud,
    types::U256Wrapper,
};
use tracing::info;

use super::utils::DepositedTrait;

impl<T: DepositedTrait + Sync> EventHandler for T {
    /// This function creates an `Event` for the `Deposited` event
    async fn create_event(
        &self,
        decoded_consumer_context: &DecodedConsumerContext,
        event: &DecodedMessage,
        deposit_id: String,
    ) -> Result<Event, ConsumerError> {
        // Create the event
        let event = if self.is_triple() {
            Event::builder()
                .id(DecodedMessage::event_id(event))
                .event_type(EventType::Deposited)
                .deposit_id(deposit_id)
                .block_number(U256Wrapper::try_from(event.block_number)?)
                .block_timestamp(event.block_timestamp)
                .transaction_hash(event.transaction_hash.clone())
                .triple_id(U256Wrapper::from(self.vaultId))
                .build()
        } else {
            Event::builder()
                .id(DecodedMessage::event_id(event))
                .event_type(EventType::Deposited)
                .deposit_id(deposit_id)
                .block_number(U256Wrapper::try_from(event.block_number)?)
                .block_timestamp(event.block_timestamp)
                .transaction_hash(event.transaction_hash.clone())
                .atom_id(U256Wrapper::from(self.vaultId))
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

    /// This function handles the creation of a `Deposit`
    async fn handle_event(
        &self,
        decoded_consumer_context: &DecodedConsumerContext,
        event: &DecodedMessage,
    ) -> Result<(), ConsumerError> {
        info!(
            "Handling deposit creation for vault {:?} and block number {:?}",
            self.get_vault_id(),
            event.block_number
        );

        // We need to process the deposit one way or another, so the accounts, vault and term
        // must be initialized. This dont need to be part of the transaction.
        let vault = self
            .initialize_accounts_and_vault(decoded_consumer_context, event)
            .await?;

        // Fetch the current share price and total shares
        let current_share_price: U256Wrapper = decoded_consumer_context
            .fetch_current_share_price(self.get_vault_id(), event.block_number)
            .await?
            .into();

        // Fetch the total shares in the vault
        let total_shares = decoded_consumer_context
            .fetch_total_shares_in_vault(self.vaultId, event.block_number)
            .await?;
        let mut tx = decoded_consumer_context.pg_pool.begin().await?;

        // Create deposit record
        let deposit = self
            .create_deposit(event, &decoded_consumer_context.backend_schema, &mut tx)
            .await?;

        // Handle position and related entities
        self.handle_position_and_claims(
            decoded_consumer_context,
            &mut tx,
            current_share_price,
            total_shares,
        )
        .await?;

        tx.commit().await?;

        // Create event
        self.create_event(decoded_consumer_context, event, deposit.id)
            .await?;

        // Create signal
        self.create_signal(decoded_consumer_context, event, &vault)
            .await?;

        Ok(())
    }
}
