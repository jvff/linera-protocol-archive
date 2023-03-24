use crate::{
    crypto::{CryptoHash, PublicKey, Signature},
    data_types::{BlockHeight, ChainDescription, ChainId, Owner, Timestamp},
};
use async_graphql::scalar;

scalar!(BlockHeight);
scalar!(ChainDescription);
scalar!(ChainId);
scalar!(CryptoHash);
scalar!(Owner);
scalar!(PublicKey);
scalar!(Signature);
scalar!(Timestamp);
