use crate::{
    error::ConsumerError, mode::types::DecodedConsumerContext, schemas::types::DecodedMessage,
    traits::EventHandler,
};
use models::{
    event::{Event, EventType},
    fee_transfer::FeeTransfer,
    traits::SimpleCrud,
    types::U256Wrapper,
};
use std::fmt::Debug;
use tracing::info;

use super::utils::FeesTransferredTrait;

impl<T: FeesTransferredTrait + Debug> EventHandler for T {
    /// This function creates an `Event` for the `FeesTransferred` event
    async fn create_event(
        &self,
        decoded_consumer_context: &DecodedConsumerContext,
        event: &DecodedMessage,
    ) -> Result<(), ConsumerError> {
        // Create the event
        Event::builder()
            .id(DecodedMessage::event_id(event))
            .event_type(EventType::FeesTransfered)
            .fee_transfer_id(DecodedMessage::event_id(event))
            .block_number(U256Wrapper::try_from(event.block_number)?)
            .block_timestamp(event.block_timestamp)
            .transaction_hash(event.transaction_hash.clone())
            .build()
            .upsert(
                &decoded_consumer_context.backend_schema,
                &decoded_consumer_context.pg_pool,
            )
            .await?;

        Ok(())
    }

    /// This function handles an `FeesTransferred` event.
    async fn handle_event(
        &self,
        decoded_consumer_context: &DecodedConsumerContext,
        event: &DecodedMessage,
    ) -> Result<(), ConsumerError> {
        info!("Handling fees transfer: {self:#?}");

        // Check if a fee transfer already exists
        if let Some(_fee_transfer) = FeeTransfer::find_by_id(
            DecodedMessage::event_id(event),
            &decoded_consumer_context.backend_schema,
            &decoded_consumer_context.pg_pool,
        )
        .await?
        {
            info!("Fee transfer already exists, skipping.");
            return Ok(());
        }

        // Get or create the sender account
        let sender_account = self
            .get_or_create_sender_account(decoded_consumer_context)
            .await?;

        // Upsert the protocol multisig account
        let protocol_multisig_account = self
            .upsert_protocol_multisig_account(decoded_consumer_context)
            .await?;

        // Create the fee transfer record
        self.create_fee_transfer(
            decoded_consumer_context,
            &sender_account,
            &protocol_multisig_account,
            event,
        )
        .await?;

        // Create the event
        self.create_event(decoded_consumer_context, event).await?;

        Ok(())
    }
}
