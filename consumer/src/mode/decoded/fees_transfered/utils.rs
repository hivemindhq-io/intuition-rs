use crate::{
    EthMultiVault::FeesTransferred,
    EthMultiVaultV1_5::FeesTransferred as FeesTransferredV1_5,
    error::ConsumerError,
    mode::{types::DecodedConsumerContext, utils::short_id},
    schemas::types::DecodedMessage,
};
use models::{
    account::{Account, AccountType},
    fee_transfer::FeeTransfer,
    traits::SimpleCrud,
    types::U256Wrapper,
};

/// Trait to unify the common functionality between FeesTransferred and FeesTransferredV1_5
pub trait FeesTransferredTrait {
    fn get_sender(&self) -> String;
    fn get_protocol_multisig(&self) -> String;
    fn get_amount(&self) -> U256Wrapper;

    /// This function creates a fee transfer record
    async fn create_fee_transfer(
        &self,
        decoded_consumer_context: &DecodedConsumerContext,
        sender_account: &Account,
        protocol_multisig_account: &Account,
        event: &DecodedMessage,
    ) -> Result<FeeTransfer, ConsumerError> {
        FeeTransfer::builder()
            .id(DecodedMessage::event_id(event))
            .sender_id(sender_account.id.clone())
            .receiver_id(protocol_multisig_account.id.clone())
            .amount(self.get_amount())
            .block_number(U256Wrapper::try_from(event.block_number)?)
            .block_timestamp(event.block_timestamp)
            .transaction_hash(event.transaction_hash.clone())
            .build()
            .upsert(
                &decoded_consumer_context.backend_schema,
                &decoded_consumer_context.pg_pool,
            )
            .await
            .map_err(ConsumerError::ModelError)
    }

    /// This function gets or creates a sender account
    async fn get_or_create_sender_account(
        &self,
        decoded_consumer_context: &DecodedConsumerContext,
    ) -> Result<Account, ConsumerError> {
        // First try to find existing account
        if let Some(account) = Account::find_by_id(
            self.get_sender(),
            &decoded_consumer_context.backend_schema,
            &decoded_consumer_context.pg_pool,
        )
        .await?
        {
            return Ok(account);
        }

        // Only create new account if none exists
        Account::builder()
            .id(self.get_sender())
            .label(short_id(&self.get_sender()))
            .account_type(AccountType::Default)
            .build()
            .upsert(
                &decoded_consumer_context.backend_schema,
                &decoded_consumer_context.pg_pool,
            )
            .await
            .map_err(ConsumerError::ModelError)
    }

    /// This function upserts the protocol multisig account
    async fn upsert_protocol_multisig_account(
        &self,
        decoded_consumer_context: &DecodedConsumerContext,
    ) -> Result<Account, ConsumerError> {
        Account::find_by_id(
            self.get_protocol_multisig(),
            &decoded_consumer_context.backend_schema,
            &decoded_consumer_context.pg_pool,
        )
        .await?
        .unwrap_or_else(|| {
            Account::builder()
                .id(self.get_protocol_multisig())
                .label("Protocol Multisig")
                .account_type(AccountType::ProtocolVault)
                .build()
        })
        .upsert(
            &decoded_consumer_context.backend_schema,
            &decoded_consumer_context.pg_pool,
        )
        .await
        .map_err(ConsumerError::ModelError)
    }
}

impl FeesTransferredTrait for FeesTransferred {
    fn get_sender(&self) -> String {
        self.sender.to_string()
    }

    fn get_protocol_multisig(&self) -> String {
        self.protocolVault.to_string()
    }

    fn get_amount(&self) -> U256Wrapper {
        U256Wrapper::from(self.amount)
    }
}

impl FeesTransferredTrait for FeesTransferredV1_5 {
    fn get_sender(&self) -> String {
        self.sender.to_string()
    }

    fn get_protocol_multisig(&self) -> String {
        self.protocolMultisig.to_string()
    }

    fn get_amount(&self) -> U256Wrapper {
        U256Wrapper::from(self.amount)
    }
}
