#[allow(unused_imports)]
use progenitor_client::{encode_path, RequestBuilderExt};
#[allow(unused_imports)]
pub use progenitor_client::{ByteStream, Error, ResponseValue};
#[allow(unused_imports)]
use reqwest::header::{HeaderMap, HeaderValue};
/// Types used as operation parameters and responses.
#[allow(clippy::all)]
pub mod types {
    /// Error types.
    pub mod error {
        /// Error from a TryFrom or FromStr implementation.
        pub struct ConversionError(::std::borrow::Cow<'static, str>);
        impl ::std::error::Error for ConversionError {}
        impl ::std::fmt::Display for ConversionError {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
                ::std::fmt::Display::fmt(&self.0, f)
            }
        }

        impl ::std::fmt::Debug for ConversionError {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
                ::std::fmt::Debug::fmt(&self.0, f)
            }
        }

        impl From<&'static str> for ConversionError {
            fn from(value: &'static str) -> Self {
                Self(value.into())
            }
        }

        impl From<String> for ConversionError {
            fn from(value: String) -> Self {
                Self(value.into())
            }
        }
    }

    ///AcceptInvitationIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "authenticator",
    ///    "invitationId",
    ///    "userId"
    ///  ],
    ///  "properties": {
    ///    "authenticator": {
    ///      "$ref": "#/components/schemas/AuthenticatorParams"
    ///    },
    ///    "invitationId": {
    ///      "description": "Unique identifier for a given Invitation object.",
    ///      "type": "string"
    ///    },
    ///    "userId": {
    ///      "description": "Unique identifier for a given User.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct AcceptInvitationIntent {
        pub authenticator: AuthenticatorParams,
        ///Unique identifier for a given Invitation object.
        #[serde(rename = "invitationId")]
        pub invitation_id: String,
        ///Unique identifier for a given User.
        #[serde(rename = "userId")]
        pub user_id: String,
    }

    impl From<&AcceptInvitationIntent> for AcceptInvitationIntent {
        fn from(value: &AcceptInvitationIntent) -> Self {
            value.clone()
        }
    }

    ///AcceptInvitationIntentV2
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "authenticator",
    ///    "invitationId",
    ///    "userId"
    ///  ],
    ///  "properties": {
    ///    "authenticator": {
    ///      "$ref": "#/components/schemas/AuthenticatorParamsV2"
    ///    },
    ///    "invitationId": {
    ///      "description": "Unique identifier for a given Invitation object.",
    ///      "type": "string"
    ///    },
    ///    "userId": {
    ///      "description": "Unique identifier for a given User.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct AcceptInvitationIntentV2 {
        pub authenticator: AuthenticatorParamsV2,
        ///Unique identifier for a given Invitation object.
        #[serde(rename = "invitationId")]
        pub invitation_id: String,
        ///Unique identifier for a given User.
        #[serde(rename = "userId")]
        pub user_id: String,
    }

    impl From<&AcceptInvitationIntentV2> for AcceptInvitationIntentV2 {
        fn from(value: &AcceptInvitationIntentV2) -> Self {
            value.clone()
        }
    }

    ///AcceptInvitationResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "invitationId",
    ///    "userId"
    ///  ],
    ///  "properties": {
    ///    "invitationId": {
    ///      "description": "Unique identifier for a given Invitation.",
    ///      "type": "string"
    ///    },
    ///    "userId": {
    ///      "description": "Unique identifier for a given User.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct AcceptInvitationResult {
        ///Unique identifier for a given Invitation.
        #[serde(rename = "invitationId")]
        pub invitation_id: String,
        ///Unique identifier for a given User.
        #[serde(rename = "userId")]
        pub user_id: String,
    }

    impl From<&AcceptInvitationResult> for AcceptInvitationResult {
        fn from(value: &AcceptInvitationResult) -> Self {
            value.clone()
        }
    }

    ///AccessType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACCESS_TYPE_WEB",
    ///    "ACCESS_TYPE_API",
    ///    "ACCESS_TYPE_ALL"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum AccessType {
        #[serde(rename = "ACCESS_TYPE_WEB")]
        AccessTypeWeb,
        #[serde(rename = "ACCESS_TYPE_API")]
        AccessTypeApi,
        #[serde(rename = "ACCESS_TYPE_ALL")]
        AccessTypeAll,
    }

    impl From<&AccessType> for AccessType {
        fn from(value: &AccessType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for AccessType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::AccessTypeWeb => write!(f, "ACCESS_TYPE_WEB"),
                Self::AccessTypeApi => write!(f, "ACCESS_TYPE_API"),
                Self::AccessTypeAll => write!(f, "ACCESS_TYPE_ALL"),
            }
        }
    }

    impl std::str::FromStr for AccessType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACCESS_TYPE_WEB" => Ok(Self::AccessTypeWeb),
                "ACCESS_TYPE_API" => Ok(Self::AccessTypeApi),
                "ACCESS_TYPE_ALL" => Ok(Self::AccessTypeAll),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for AccessType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for AccessType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for AccessType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///ActivateBillingTierIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "productId"
    ///  ],
    ///  "properties": {
    ///    "productId": {
    ///      "description": "The product that the customer wants to subscribe
    /// to.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ActivateBillingTierIntent {
        ///The product that the customer wants to subscribe to.
        #[serde(rename = "productId")]
        pub product_id: String,
    }

    impl From<&ActivateBillingTierIntent> for ActivateBillingTierIntent {
        fn from(value: &ActivateBillingTierIntent) -> Self {
            value.clone()
        }
    }

    ///ActivateBillingTierResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "productId"
    ///  ],
    ///  "properties": {
    ///    "productId": {
    ///      "description": "The id of the product being subscribed to.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ActivateBillingTierResult {
        ///The id of the product being subscribed to.
        #[serde(rename = "productId")]
        pub product_id: String,
    }

    impl From<&ActivateBillingTierResult> for ActivateBillingTierResult {
        fn from(value: &ActivateBillingTierResult) -> Self {
            value.clone()
        }
    }

    ///Activity
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "canApprove",
    ///    "canReject",
    ///    "createdAt",
    ///    "fingerprint",
    ///    "id",
    ///    "intent",
    ///    "organizationId",
    ///    "result",
    ///    "status",
    ///    "type",
    ///    "updatedAt",
    ///    "votes"
    ///  ],
    ///  "properties": {
    ///    "canApprove": {
    ///      "type": "boolean"
    ///    },
    ///    "canReject": {
    ///      "type": "boolean"
    ///    },
    ///    "createdAt": {
    ///      "$ref": "#/components/schemas/external.data.v1.Timestamp"
    ///    },
    ///    "failure": {
    ///      "$ref": "#/components/schemas/Status"
    ///    },
    ///    "fingerprint": {
    ///      "description": "An artifact verifying a User's action.",
    ///      "type": "string"
    ///    },
    ///    "id": {
    ///      "description": "Unique identifier for a given Activity object.",
    ///      "type": "string"
    ///    },
    ///    "intent": {
    ///      "$ref": "#/components/schemas/Intent"
    ///    },
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "result": {
    ///      "$ref": "#/components/schemas/Result"
    ///    },
    ///    "status": {
    ///      "$ref": "#/components/schemas/ActivityStatus"
    ///    },
    ///    "type": {
    ///      "$ref": "#/components/schemas/ActivityType"
    ///    },
    ///    "updatedAt": {
    ///      "$ref": "#/components/schemas/external.data.v1.Timestamp"
    ///    },
    ///    "votes": {
    ///      "description": "A list of objects representing a particular User's
    /// approval or rejection of a Consensus request, including all relevant
    /// metadata.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/Vote"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct Activity {
        #[serde(rename = "canApprove")]
        pub can_approve: bool,
        #[serde(rename = "canReject")]
        pub can_reject: bool,
        #[serde(rename = "createdAt")]
        pub created_at: ExternalDataV1Timestamp,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub failure: Option<Status>,
        ///An artifact verifying a User's action.
        pub fingerprint: String,
        ///Unique identifier for a given Activity object.
        pub id: String,
        pub intent: Intent,
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub result: Result,
        pub status: ActivityStatus,
        #[serde(rename = "type")]
        pub type_: ActivityType,
        #[serde(rename = "updatedAt")]
        pub updated_at: ExternalDataV1Timestamp,
        ///A list of objects representing a particular User's approval or
        /// rejection of a Consensus request, including all relevant metadata.
        pub votes: Vec<Vote>,
    }

    impl From<&Activity> for Activity {
        fn from(value: &Activity) -> Self {
            value.clone()
        }
    }

    ///ActivityResponse
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "activity"
    ///  ],
    ///  "properties": {
    ///    "activity": {
    ///      "$ref": "#/components/schemas/Activity"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ActivityResponse {
        pub activity: Activity,
    }

    impl From<&ActivityResponse> for ActivityResponse {
        fn from(value: &ActivityResponse) -> Self {
            value.clone()
        }
    }

    ///ActivityStatus
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_STATUS_CREATED",
    ///    "ACTIVITY_STATUS_PENDING",
    ///    "ACTIVITY_STATUS_COMPLETED",
    ///    "ACTIVITY_STATUS_FAILED",
    ///    "ACTIVITY_STATUS_CONSENSUS_NEEDED",
    ///    "ACTIVITY_STATUS_REJECTED"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum ActivityStatus {
        #[serde(rename = "ACTIVITY_STATUS_CREATED")]
        ActivityStatusCreated,
        #[serde(rename = "ACTIVITY_STATUS_PENDING")]
        ActivityStatusPending,
        #[serde(rename = "ACTIVITY_STATUS_COMPLETED")]
        ActivityStatusCompleted,
        #[serde(rename = "ACTIVITY_STATUS_FAILED")]
        ActivityStatusFailed,
        #[serde(rename = "ACTIVITY_STATUS_CONSENSUS_NEEDED")]
        ActivityStatusConsensusNeeded,
        #[serde(rename = "ACTIVITY_STATUS_REJECTED")]
        ActivityStatusRejected,
    }

    impl From<&ActivityStatus> for ActivityStatus {
        fn from(value: &ActivityStatus) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for ActivityStatus {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityStatusCreated => write!(f, "ACTIVITY_STATUS_CREATED"),
                Self::ActivityStatusPending => write!(f, "ACTIVITY_STATUS_PENDING"),
                Self::ActivityStatusCompleted => write!(f, "ACTIVITY_STATUS_COMPLETED"),
                Self::ActivityStatusFailed => write!(f, "ACTIVITY_STATUS_FAILED"),
                Self::ActivityStatusConsensusNeeded => {
                    write!(f, "ACTIVITY_STATUS_CONSENSUS_NEEDED")
                }
                Self::ActivityStatusRejected => write!(f, "ACTIVITY_STATUS_REJECTED"),
            }
        }
    }

    impl std::str::FromStr for ActivityStatus {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_STATUS_CREATED" => Ok(Self::ActivityStatusCreated),
                "ACTIVITY_STATUS_PENDING" => Ok(Self::ActivityStatusPending),
                "ACTIVITY_STATUS_COMPLETED" => Ok(Self::ActivityStatusCompleted),
                "ACTIVITY_STATUS_FAILED" => Ok(Self::ActivityStatusFailed),
                "ACTIVITY_STATUS_CONSENSUS_NEEDED" => Ok(Self::ActivityStatusConsensusNeeded),
                "ACTIVITY_STATUS_REJECTED" => Ok(Self::ActivityStatusRejected),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for ActivityStatus {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for ActivityStatus {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for ActivityStatus {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///ActivityType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_CREATE_API_KEYS",
    ///    "ACTIVITY_TYPE_CREATE_USERS",
    ///    "ACTIVITY_TYPE_CREATE_PRIVATE_KEYS",
    ///    "ACTIVITY_TYPE_SIGN_RAW_PAYLOAD",
    ///    "ACTIVITY_TYPE_CREATE_INVITATIONS",
    ///    "ACTIVITY_TYPE_ACCEPT_INVITATION",
    ///    "ACTIVITY_TYPE_CREATE_POLICY",
    ///    "ACTIVITY_TYPE_DISABLE_PRIVATE_KEY",
    ///    "ACTIVITY_TYPE_DELETE_USERS",
    ///    "ACTIVITY_TYPE_DELETE_API_KEYS",
    ///    "ACTIVITY_TYPE_DELETE_INVITATION",
    ///    "ACTIVITY_TYPE_DELETE_ORGANIZATION",
    ///    "ACTIVITY_TYPE_DELETE_POLICY",
    ///    "ACTIVITY_TYPE_CREATE_USER_TAG",
    ///    "ACTIVITY_TYPE_DELETE_USER_TAGS",
    ///    "ACTIVITY_TYPE_CREATE_ORGANIZATION",
    ///    "ACTIVITY_TYPE_SIGN_TRANSACTION",
    ///    "ACTIVITY_TYPE_APPROVE_ACTIVITY",
    ///    "ACTIVITY_TYPE_REJECT_ACTIVITY",
    ///    "ACTIVITY_TYPE_DELETE_AUTHENTICATORS",
    ///    "ACTIVITY_TYPE_CREATE_AUTHENTICATORS",
    ///    "ACTIVITY_TYPE_CREATE_PRIVATE_KEY_TAG",
    ///    "ACTIVITY_TYPE_DELETE_PRIVATE_KEY_TAGS",
    ///    "ACTIVITY_TYPE_SET_PAYMENT_METHOD",
    ///    "ACTIVITY_TYPE_ACTIVATE_BILLING_TIER",
    ///    "ACTIVITY_TYPE_DELETE_PAYMENT_METHOD",
    ///    "ACTIVITY_TYPE_CREATE_POLICY_V2",
    ///    "ACTIVITY_TYPE_CREATE_POLICY_V3",
    ///    "ACTIVITY_TYPE_CREATE_API_ONLY_USERS",
    ///    "ACTIVITY_TYPE_UPDATE_ROOT_QUORUM",
    ///    "ACTIVITY_TYPE_UPDATE_USER_TAG",
    ///    "ACTIVITY_TYPE_UPDATE_PRIVATE_KEY_TAG",
    ///    "ACTIVITY_TYPE_CREATE_AUTHENTICATORS_V2",
    ///    "ACTIVITY_TYPE_CREATE_ORGANIZATION_V2",
    ///    "ACTIVITY_TYPE_CREATE_USERS_V2",
    ///    "ACTIVITY_TYPE_ACCEPT_INVITATION_V2",
    ///    "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION",
    ///    "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V2",
    ///    "ACTIVITY_TYPE_UPDATE_ALLOWED_ORIGINS",
    ///    "ACTIVITY_TYPE_CREATE_PRIVATE_KEYS_V2",
    ///    "ACTIVITY_TYPE_UPDATE_USER",
    ///    "ACTIVITY_TYPE_UPDATE_POLICY",
    ///    "ACTIVITY_TYPE_SET_PAYMENT_METHOD_V2",
    ///    "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V3",
    ///    "ACTIVITY_TYPE_CREATE_WALLET",
    ///    "ACTIVITY_TYPE_CREATE_WALLET_ACCOUNTS",
    ///    "ACTIVITY_TYPE_INIT_USER_EMAIL_RECOVERY",
    ///    "ACTIVITY_TYPE_RECOVER_USER",
    ///    "ACTIVITY_TYPE_SET_ORGANIZATION_FEATURE",
    ///    "ACTIVITY_TYPE_REMOVE_ORGANIZATION_FEATURE",
    ///    "ACTIVITY_TYPE_SIGN_RAW_PAYLOAD_V2",
    ///    "ACTIVITY_TYPE_SIGN_TRANSACTION_V2",
    ///    "ACTIVITY_TYPE_EXPORT_PRIVATE_KEY",
    ///    "ACTIVITY_TYPE_EXPORT_WALLET",
    ///    "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V4",
    ///    "ACTIVITY_TYPE_EMAIL_AUTH",
    ///    "ACTIVITY_TYPE_EXPORT_WALLET_ACCOUNT",
    ///    "ACTIVITY_TYPE_INIT_IMPORT_WALLET",
    ///    "ACTIVITY_TYPE_IMPORT_WALLET",
    ///    "ACTIVITY_TYPE_INIT_IMPORT_PRIVATE_KEY",
    ///    "ACTIVITY_TYPE_IMPORT_PRIVATE_KEY",
    ///    "ACTIVITY_TYPE_CREATE_POLICIES",
    ///    "ACTIVITY_TYPE_SIGN_RAW_PAYLOADS",
    ///    "ACTIVITY_TYPE_CREATE_READ_ONLY_SESSION",
    ///    "ACTIVITY_TYPE_CREATE_OAUTH_PROVIDERS",
    ///    "ACTIVITY_TYPE_DELETE_OAUTH_PROVIDERS",
    ///    "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V5",
    ///    "ACTIVITY_TYPE_OAUTH",
    ///    "ACTIVITY_TYPE_CREATE_API_KEYS_V2",
    ///    "ACTIVITY_TYPE_CREATE_READ_WRITE_SESSION",
    ///    "ACTIVITY_TYPE_EMAIL_AUTH_V2",
    ///    "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V6",
    ///    "ACTIVITY_TYPE_DELETE_PRIVATE_KEYS",
    ///    "ACTIVITY_TYPE_DELETE_WALLETS",
    ///    "ACTIVITY_TYPE_CREATE_READ_WRITE_SESSION_V2",
    ///    "ACTIVITY_TYPE_DELETE_SUB_ORGANIZATION",
    ///    "ACTIVITY_TYPE_INIT_OTP_AUTH",
    ///    "ACTIVITY_TYPE_OTP_AUTH",
    ///    "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V7"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum ActivityType {
        #[serde(rename = "ACTIVITY_TYPE_CREATE_API_KEYS")]
        ActivityTypeCreateApiKeys,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_USERS")]
        ActivityTypeCreateUsers,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_PRIVATE_KEYS")]
        ActivityTypeCreatePrivateKeys,
        #[serde(rename = "ACTIVITY_TYPE_SIGN_RAW_PAYLOAD")]
        ActivityTypeSignRawPayload,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_INVITATIONS")]
        ActivityTypeCreateInvitations,
        #[serde(rename = "ACTIVITY_TYPE_ACCEPT_INVITATION")]
        ActivityTypeAcceptInvitation,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_POLICY")]
        ActivityTypeCreatePolicy,
        #[serde(rename = "ACTIVITY_TYPE_DISABLE_PRIVATE_KEY")]
        ActivityTypeDisablePrivateKey,
        #[serde(rename = "ACTIVITY_TYPE_DELETE_USERS")]
        ActivityTypeDeleteUsers,
        #[serde(rename = "ACTIVITY_TYPE_DELETE_API_KEYS")]
        ActivityTypeDeleteApiKeys,
        #[serde(rename = "ACTIVITY_TYPE_DELETE_INVITATION")]
        ActivityTypeDeleteInvitation,
        #[serde(rename = "ACTIVITY_TYPE_DELETE_ORGANIZATION")]
        ActivityTypeDeleteOrganization,
        #[serde(rename = "ACTIVITY_TYPE_DELETE_POLICY")]
        ActivityTypeDeletePolicy,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_USER_TAG")]
        ActivityTypeCreateUserTag,
        #[serde(rename = "ACTIVITY_TYPE_DELETE_USER_TAGS")]
        ActivityTypeDeleteUserTags,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_ORGANIZATION")]
        ActivityTypeCreateOrganization,
        #[serde(rename = "ACTIVITY_TYPE_SIGN_TRANSACTION")]
        ActivityTypeSignTransaction,
        #[serde(rename = "ACTIVITY_TYPE_APPROVE_ACTIVITY")]
        ActivityTypeApproveActivity,
        #[serde(rename = "ACTIVITY_TYPE_REJECT_ACTIVITY")]
        ActivityTypeRejectActivity,
        #[serde(rename = "ACTIVITY_TYPE_DELETE_AUTHENTICATORS")]
        ActivityTypeDeleteAuthenticators,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_AUTHENTICATORS")]
        ActivityTypeCreateAuthenticators,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_PRIVATE_KEY_TAG")]
        ActivityTypeCreatePrivateKeyTag,
        #[serde(rename = "ACTIVITY_TYPE_DELETE_PRIVATE_KEY_TAGS")]
        ActivityTypeDeletePrivateKeyTags,
        #[serde(rename = "ACTIVITY_TYPE_SET_PAYMENT_METHOD")]
        ActivityTypeSetPaymentMethod,
        #[serde(rename = "ACTIVITY_TYPE_ACTIVATE_BILLING_TIER")]
        ActivityTypeActivateBillingTier,
        #[serde(rename = "ACTIVITY_TYPE_DELETE_PAYMENT_METHOD")]
        ActivityTypeDeletePaymentMethod,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_POLICY_V2")]
        ActivityTypeCreatePolicyV2,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_POLICY_V3")]
        ActivityTypeCreatePolicyV3,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_API_ONLY_USERS")]
        ActivityTypeCreateApiOnlyUsers,
        #[serde(rename = "ACTIVITY_TYPE_UPDATE_ROOT_QUORUM")]
        ActivityTypeUpdateRootQuorum,
        #[serde(rename = "ACTIVITY_TYPE_UPDATE_USER_TAG")]
        ActivityTypeUpdateUserTag,
        #[serde(rename = "ACTIVITY_TYPE_UPDATE_PRIVATE_KEY_TAG")]
        ActivityTypeUpdatePrivateKeyTag,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_AUTHENTICATORS_V2")]
        ActivityTypeCreateAuthenticatorsV2,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_ORGANIZATION_V2")]
        ActivityTypeCreateOrganizationV2,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_USERS_V2")]
        ActivityTypeCreateUsersV2,
        #[serde(rename = "ACTIVITY_TYPE_ACCEPT_INVITATION_V2")]
        ActivityTypeAcceptInvitationV2,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION")]
        ActivityTypeCreateSubOrganization,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V2")]
        ActivityTypeCreateSubOrganizationV2,
        #[serde(rename = "ACTIVITY_TYPE_UPDATE_ALLOWED_ORIGINS")]
        ActivityTypeUpdateAllowedOrigins,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_PRIVATE_KEYS_V2")]
        ActivityTypeCreatePrivateKeysV2,
        #[serde(rename = "ACTIVITY_TYPE_UPDATE_USER")]
        ActivityTypeUpdateUser,
        #[serde(rename = "ACTIVITY_TYPE_UPDATE_POLICY")]
        ActivityTypeUpdatePolicy,
        #[serde(rename = "ACTIVITY_TYPE_SET_PAYMENT_METHOD_V2")]
        ActivityTypeSetPaymentMethodV2,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V3")]
        ActivityTypeCreateSubOrganizationV3,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_WALLET")]
        ActivityTypeCreateWallet,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_WALLET_ACCOUNTS")]
        ActivityTypeCreateWalletAccounts,
        #[serde(rename = "ACTIVITY_TYPE_INIT_USER_EMAIL_RECOVERY")]
        ActivityTypeInitUserEmailRecovery,
        #[serde(rename = "ACTIVITY_TYPE_RECOVER_USER")]
        ActivityTypeRecoverUser,
        #[serde(rename = "ACTIVITY_TYPE_SET_ORGANIZATION_FEATURE")]
        ActivityTypeSetOrganizationFeature,
        #[serde(rename = "ACTIVITY_TYPE_REMOVE_ORGANIZATION_FEATURE")]
        ActivityTypeRemoveOrganizationFeature,
        #[serde(rename = "ACTIVITY_TYPE_SIGN_RAW_PAYLOAD_V2")]
        ActivityTypeSignRawPayloadV2,
        #[serde(rename = "ACTIVITY_TYPE_SIGN_TRANSACTION_V2")]
        ActivityTypeSignTransactionV2,
        #[serde(rename = "ACTIVITY_TYPE_EXPORT_PRIVATE_KEY")]
        ActivityTypeExportPrivateKey,
        #[serde(rename = "ACTIVITY_TYPE_EXPORT_WALLET")]
        ActivityTypeExportWallet,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V4")]
        ActivityTypeCreateSubOrganizationV4,
        #[serde(rename = "ACTIVITY_TYPE_EMAIL_AUTH")]
        ActivityTypeEmailAuth,
        #[serde(rename = "ACTIVITY_TYPE_EXPORT_WALLET_ACCOUNT")]
        ActivityTypeExportWalletAccount,
        #[serde(rename = "ACTIVITY_TYPE_INIT_IMPORT_WALLET")]
        ActivityTypeInitImportWallet,
        #[serde(rename = "ACTIVITY_TYPE_IMPORT_WALLET")]
        ActivityTypeImportWallet,
        #[serde(rename = "ACTIVITY_TYPE_INIT_IMPORT_PRIVATE_KEY")]
        ActivityTypeInitImportPrivateKey,
        #[serde(rename = "ACTIVITY_TYPE_IMPORT_PRIVATE_KEY")]
        ActivityTypeImportPrivateKey,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_POLICIES")]
        ActivityTypeCreatePolicies,
        #[serde(rename = "ACTIVITY_TYPE_SIGN_RAW_PAYLOADS")]
        ActivityTypeSignRawPayloads,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_READ_ONLY_SESSION")]
        ActivityTypeCreateReadOnlySession,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_OAUTH_PROVIDERS")]
        ActivityTypeCreateOauthProviders,
        #[serde(rename = "ACTIVITY_TYPE_DELETE_OAUTH_PROVIDERS")]
        ActivityTypeDeleteOauthProviders,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V5")]
        ActivityTypeCreateSubOrganizationV5,
        #[serde(rename = "ACTIVITY_TYPE_OAUTH")]
        ActivityTypeOauth,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_API_KEYS_V2")]
        ActivityTypeCreateApiKeysV2,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_READ_WRITE_SESSION")]
        ActivityTypeCreateReadWriteSession,
        #[serde(rename = "ACTIVITY_TYPE_EMAIL_AUTH_V2")]
        ActivityTypeEmailAuthV2,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V6")]
        ActivityTypeCreateSubOrganizationV6,
        #[serde(rename = "ACTIVITY_TYPE_DELETE_PRIVATE_KEYS")]
        ActivityTypeDeletePrivateKeys,
        #[serde(rename = "ACTIVITY_TYPE_DELETE_WALLETS")]
        ActivityTypeDeleteWallets,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_READ_WRITE_SESSION_V2")]
        ActivityTypeCreateReadWriteSessionV2,
        #[serde(rename = "ACTIVITY_TYPE_DELETE_SUB_ORGANIZATION")]
        ActivityTypeDeleteSubOrganization,
        #[serde(rename = "ACTIVITY_TYPE_INIT_OTP_AUTH")]
        ActivityTypeInitOtpAuth,
        #[serde(rename = "ACTIVITY_TYPE_OTP_AUTH")]
        ActivityTypeOtpAuth,
        #[serde(rename = "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V7")]
        ActivityTypeCreateSubOrganizationV7,
    }

    impl From<&ActivityType> for ActivityType {
        fn from(value: &ActivityType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for ActivityType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeCreateApiKeys => write!(f, "ACTIVITY_TYPE_CREATE_API_KEYS"),
                Self::ActivityTypeCreateUsers => write!(f, "ACTIVITY_TYPE_CREATE_USERS"),
                Self::ActivityTypeCreatePrivateKeys => {
                    write!(f, "ACTIVITY_TYPE_CREATE_PRIVATE_KEYS")
                }
                Self::ActivityTypeSignRawPayload => write!(f, "ACTIVITY_TYPE_SIGN_RAW_PAYLOAD"),
                Self::ActivityTypeCreateInvitations => {
                    write!(f, "ACTIVITY_TYPE_CREATE_INVITATIONS")
                }
                Self::ActivityTypeAcceptInvitation => write!(f, "ACTIVITY_TYPE_ACCEPT_INVITATION"),
                Self::ActivityTypeCreatePolicy => write!(f, "ACTIVITY_TYPE_CREATE_POLICY"),
                Self::ActivityTypeDisablePrivateKey => {
                    write!(f, "ACTIVITY_TYPE_DISABLE_PRIVATE_KEY")
                }
                Self::ActivityTypeDeleteUsers => write!(f, "ACTIVITY_TYPE_DELETE_USERS"),
                Self::ActivityTypeDeleteApiKeys => write!(f, "ACTIVITY_TYPE_DELETE_API_KEYS"),
                Self::ActivityTypeDeleteInvitation => write!(f, "ACTIVITY_TYPE_DELETE_INVITATION"),
                Self::ActivityTypeDeleteOrganization => {
                    write!(f, "ACTIVITY_TYPE_DELETE_ORGANIZATION")
                }
                Self::ActivityTypeDeletePolicy => write!(f, "ACTIVITY_TYPE_DELETE_POLICY"),
                Self::ActivityTypeCreateUserTag => write!(f, "ACTIVITY_TYPE_CREATE_USER_TAG"),
                Self::ActivityTypeDeleteUserTags => write!(f, "ACTIVITY_TYPE_DELETE_USER_TAGS"),
                Self::ActivityTypeCreateOrganization => {
                    write!(f, "ACTIVITY_TYPE_CREATE_ORGANIZATION")
                }
                Self::ActivityTypeSignTransaction => write!(f, "ACTIVITY_TYPE_SIGN_TRANSACTION"),
                Self::ActivityTypeApproveActivity => write!(f, "ACTIVITY_TYPE_APPROVE_ACTIVITY"),
                Self::ActivityTypeRejectActivity => write!(f, "ACTIVITY_TYPE_REJECT_ACTIVITY"),
                Self::ActivityTypeDeleteAuthenticators => {
                    write!(f, "ACTIVITY_TYPE_DELETE_AUTHENTICATORS")
                }
                Self::ActivityTypeCreateAuthenticators => {
                    write!(f, "ACTIVITY_TYPE_CREATE_AUTHENTICATORS")
                }
                Self::ActivityTypeCreatePrivateKeyTag => {
                    write!(f, "ACTIVITY_TYPE_CREATE_PRIVATE_KEY_TAG")
                }
                Self::ActivityTypeDeletePrivateKeyTags => {
                    write!(f, "ACTIVITY_TYPE_DELETE_PRIVATE_KEY_TAGS")
                }
                Self::ActivityTypeSetPaymentMethod => write!(f, "ACTIVITY_TYPE_SET_PAYMENT_METHOD"),
                Self::ActivityTypeActivateBillingTier => {
                    write!(f, "ACTIVITY_TYPE_ACTIVATE_BILLING_TIER")
                }
                Self::ActivityTypeDeletePaymentMethod => {
                    write!(f, "ACTIVITY_TYPE_DELETE_PAYMENT_METHOD")
                }
                Self::ActivityTypeCreatePolicyV2 => write!(f, "ACTIVITY_TYPE_CREATE_POLICY_V2"),
                Self::ActivityTypeCreatePolicyV3 => write!(f, "ACTIVITY_TYPE_CREATE_POLICY_V3"),
                Self::ActivityTypeCreateApiOnlyUsers => {
                    write!(f, "ACTIVITY_TYPE_CREATE_API_ONLY_USERS")
                }
                Self::ActivityTypeUpdateRootQuorum => write!(f, "ACTIVITY_TYPE_UPDATE_ROOT_QUORUM"),
                Self::ActivityTypeUpdateUserTag => write!(f, "ACTIVITY_TYPE_UPDATE_USER_TAG"),
                Self::ActivityTypeUpdatePrivateKeyTag => {
                    write!(f, "ACTIVITY_TYPE_UPDATE_PRIVATE_KEY_TAG")
                }
                Self::ActivityTypeCreateAuthenticatorsV2 => {
                    write!(f, "ACTIVITY_TYPE_CREATE_AUTHENTICATORS_V2")
                }
                Self::ActivityTypeCreateOrganizationV2 => {
                    write!(f, "ACTIVITY_TYPE_CREATE_ORGANIZATION_V2")
                }
                Self::ActivityTypeCreateUsersV2 => write!(f, "ACTIVITY_TYPE_CREATE_USERS_V2"),
                Self::ActivityTypeAcceptInvitationV2 => {
                    write!(f, "ACTIVITY_TYPE_ACCEPT_INVITATION_V2")
                }
                Self::ActivityTypeCreateSubOrganization => {
                    write!(f, "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION")
                }
                Self::ActivityTypeCreateSubOrganizationV2 => {
                    write!(f, "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V2")
                }
                Self::ActivityTypeUpdateAllowedOrigins => {
                    write!(f, "ACTIVITY_TYPE_UPDATE_ALLOWED_ORIGINS")
                }
                Self::ActivityTypeCreatePrivateKeysV2 => {
                    write!(f, "ACTIVITY_TYPE_CREATE_PRIVATE_KEYS_V2")
                }
                Self::ActivityTypeUpdateUser => write!(f, "ACTIVITY_TYPE_UPDATE_USER"),
                Self::ActivityTypeUpdatePolicy => write!(f, "ACTIVITY_TYPE_UPDATE_POLICY"),
                Self::ActivityTypeSetPaymentMethodV2 => {
                    write!(f, "ACTIVITY_TYPE_SET_PAYMENT_METHOD_V2")
                }
                Self::ActivityTypeCreateSubOrganizationV3 => {
                    write!(f, "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V3")
                }
                Self::ActivityTypeCreateWallet => write!(f, "ACTIVITY_TYPE_CREATE_WALLET"),
                Self::ActivityTypeCreateWalletAccounts => {
                    write!(f, "ACTIVITY_TYPE_CREATE_WALLET_ACCOUNTS")
                }
                Self::ActivityTypeInitUserEmailRecovery => {
                    write!(f, "ACTIVITY_TYPE_INIT_USER_EMAIL_RECOVERY")
                }
                Self::ActivityTypeRecoverUser => write!(f, "ACTIVITY_TYPE_RECOVER_USER"),
                Self::ActivityTypeSetOrganizationFeature => {
                    write!(f, "ACTIVITY_TYPE_SET_ORGANIZATION_FEATURE")
                }
                Self::ActivityTypeRemoveOrganizationFeature => {
                    write!(f, "ACTIVITY_TYPE_REMOVE_ORGANIZATION_FEATURE")
                }
                Self::ActivityTypeSignRawPayloadV2 => {
                    write!(f, "ACTIVITY_TYPE_SIGN_RAW_PAYLOAD_V2")
                }
                Self::ActivityTypeSignTransactionV2 => {
                    write!(f, "ACTIVITY_TYPE_SIGN_TRANSACTION_V2")
                }
                Self::ActivityTypeExportPrivateKey => write!(f, "ACTIVITY_TYPE_EXPORT_PRIVATE_KEY"),
                Self::ActivityTypeExportWallet => write!(f, "ACTIVITY_TYPE_EXPORT_WALLET"),
                Self::ActivityTypeCreateSubOrganizationV4 => {
                    write!(f, "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V4")
                }
                Self::ActivityTypeEmailAuth => write!(f, "ACTIVITY_TYPE_EMAIL_AUTH"),
                Self::ActivityTypeExportWalletAccount => {
                    write!(f, "ACTIVITY_TYPE_EXPORT_WALLET_ACCOUNT")
                }
                Self::ActivityTypeInitImportWallet => write!(f, "ACTIVITY_TYPE_INIT_IMPORT_WALLET"),
                Self::ActivityTypeImportWallet => write!(f, "ACTIVITY_TYPE_IMPORT_WALLET"),
                Self::ActivityTypeInitImportPrivateKey => {
                    write!(f, "ACTIVITY_TYPE_INIT_IMPORT_PRIVATE_KEY")
                }
                Self::ActivityTypeImportPrivateKey => write!(f, "ACTIVITY_TYPE_IMPORT_PRIVATE_KEY"),
                Self::ActivityTypeCreatePolicies => write!(f, "ACTIVITY_TYPE_CREATE_POLICIES"),
                Self::ActivityTypeSignRawPayloads => write!(f, "ACTIVITY_TYPE_SIGN_RAW_PAYLOADS"),
                Self::ActivityTypeCreateReadOnlySession => {
                    write!(f, "ACTIVITY_TYPE_CREATE_READ_ONLY_SESSION")
                }
                Self::ActivityTypeCreateOauthProviders => {
                    write!(f, "ACTIVITY_TYPE_CREATE_OAUTH_PROVIDERS")
                }
                Self::ActivityTypeDeleteOauthProviders => {
                    write!(f, "ACTIVITY_TYPE_DELETE_OAUTH_PROVIDERS")
                }
                Self::ActivityTypeCreateSubOrganizationV5 => {
                    write!(f, "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V5")
                }
                Self::ActivityTypeOauth => write!(f, "ACTIVITY_TYPE_OAUTH"),
                Self::ActivityTypeCreateApiKeysV2 => write!(f, "ACTIVITY_TYPE_CREATE_API_KEYS_V2"),
                Self::ActivityTypeCreateReadWriteSession => {
                    write!(f, "ACTIVITY_TYPE_CREATE_READ_WRITE_SESSION")
                }
                Self::ActivityTypeEmailAuthV2 => write!(f, "ACTIVITY_TYPE_EMAIL_AUTH_V2"),
                Self::ActivityTypeCreateSubOrganizationV6 => {
                    write!(f, "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V6")
                }
                Self::ActivityTypeDeletePrivateKeys => {
                    write!(f, "ACTIVITY_TYPE_DELETE_PRIVATE_KEYS")
                }
                Self::ActivityTypeDeleteWallets => write!(f, "ACTIVITY_TYPE_DELETE_WALLETS"),
                Self::ActivityTypeCreateReadWriteSessionV2 => {
                    write!(f, "ACTIVITY_TYPE_CREATE_READ_WRITE_SESSION_V2")
                }
                Self::ActivityTypeDeleteSubOrganization => {
                    write!(f, "ACTIVITY_TYPE_DELETE_SUB_ORGANIZATION")
                }
                Self::ActivityTypeInitOtpAuth => write!(f, "ACTIVITY_TYPE_INIT_OTP_AUTH"),
                Self::ActivityTypeOtpAuth => write!(f, "ACTIVITY_TYPE_OTP_AUTH"),
                Self::ActivityTypeCreateSubOrganizationV7 => {
                    write!(f, "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V7")
                }
            }
        }
    }

    impl std::str::FromStr for ActivityType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_CREATE_API_KEYS" => Ok(Self::ActivityTypeCreateApiKeys),
                "ACTIVITY_TYPE_CREATE_USERS" => Ok(Self::ActivityTypeCreateUsers),
                "ACTIVITY_TYPE_CREATE_PRIVATE_KEYS" => Ok(Self::ActivityTypeCreatePrivateKeys),
                "ACTIVITY_TYPE_SIGN_RAW_PAYLOAD" => Ok(Self::ActivityTypeSignRawPayload),
                "ACTIVITY_TYPE_CREATE_INVITATIONS" => Ok(Self::ActivityTypeCreateInvitations),
                "ACTIVITY_TYPE_ACCEPT_INVITATION" => Ok(Self::ActivityTypeAcceptInvitation),
                "ACTIVITY_TYPE_CREATE_POLICY" => Ok(Self::ActivityTypeCreatePolicy),
                "ACTIVITY_TYPE_DISABLE_PRIVATE_KEY" => Ok(Self::ActivityTypeDisablePrivateKey),
                "ACTIVITY_TYPE_DELETE_USERS" => Ok(Self::ActivityTypeDeleteUsers),
                "ACTIVITY_TYPE_DELETE_API_KEYS" => Ok(Self::ActivityTypeDeleteApiKeys),
                "ACTIVITY_TYPE_DELETE_INVITATION" => Ok(Self::ActivityTypeDeleteInvitation),
                "ACTIVITY_TYPE_DELETE_ORGANIZATION" => Ok(Self::ActivityTypeDeleteOrganization),
                "ACTIVITY_TYPE_DELETE_POLICY" => Ok(Self::ActivityTypeDeletePolicy),
                "ACTIVITY_TYPE_CREATE_USER_TAG" => Ok(Self::ActivityTypeCreateUserTag),
                "ACTIVITY_TYPE_DELETE_USER_TAGS" => Ok(Self::ActivityTypeDeleteUserTags),
                "ACTIVITY_TYPE_CREATE_ORGANIZATION" => Ok(Self::ActivityTypeCreateOrganization),
                "ACTIVITY_TYPE_SIGN_TRANSACTION" => Ok(Self::ActivityTypeSignTransaction),
                "ACTIVITY_TYPE_APPROVE_ACTIVITY" => Ok(Self::ActivityTypeApproveActivity),
                "ACTIVITY_TYPE_REJECT_ACTIVITY" => Ok(Self::ActivityTypeRejectActivity),
                "ACTIVITY_TYPE_DELETE_AUTHENTICATORS" => Ok(Self::ActivityTypeDeleteAuthenticators),
                "ACTIVITY_TYPE_CREATE_AUTHENTICATORS" => Ok(Self::ActivityTypeCreateAuthenticators),
                "ACTIVITY_TYPE_CREATE_PRIVATE_KEY_TAG" => Ok(Self::ActivityTypeCreatePrivateKeyTag),
                "ACTIVITY_TYPE_DELETE_PRIVATE_KEY_TAGS" => {
                    Ok(Self::ActivityTypeDeletePrivateKeyTags)
                }
                "ACTIVITY_TYPE_SET_PAYMENT_METHOD" => Ok(Self::ActivityTypeSetPaymentMethod),
                "ACTIVITY_TYPE_ACTIVATE_BILLING_TIER" => Ok(Self::ActivityTypeActivateBillingTier),
                "ACTIVITY_TYPE_DELETE_PAYMENT_METHOD" => Ok(Self::ActivityTypeDeletePaymentMethod),
                "ACTIVITY_TYPE_CREATE_POLICY_V2" => Ok(Self::ActivityTypeCreatePolicyV2),
                "ACTIVITY_TYPE_CREATE_POLICY_V3" => Ok(Self::ActivityTypeCreatePolicyV3),
                "ACTIVITY_TYPE_CREATE_API_ONLY_USERS" => Ok(Self::ActivityTypeCreateApiOnlyUsers),
                "ACTIVITY_TYPE_UPDATE_ROOT_QUORUM" => Ok(Self::ActivityTypeUpdateRootQuorum),
                "ACTIVITY_TYPE_UPDATE_USER_TAG" => Ok(Self::ActivityTypeUpdateUserTag),
                "ACTIVITY_TYPE_UPDATE_PRIVATE_KEY_TAG" => Ok(Self::ActivityTypeUpdatePrivateKeyTag),
                "ACTIVITY_TYPE_CREATE_AUTHENTICATORS_V2" => {
                    Ok(Self::ActivityTypeCreateAuthenticatorsV2)
                }
                "ACTIVITY_TYPE_CREATE_ORGANIZATION_V2" => {
                    Ok(Self::ActivityTypeCreateOrganizationV2)
                }
                "ACTIVITY_TYPE_CREATE_USERS_V2" => Ok(Self::ActivityTypeCreateUsersV2),
                "ACTIVITY_TYPE_ACCEPT_INVITATION_V2" => Ok(Self::ActivityTypeAcceptInvitationV2),
                "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION" => {
                    Ok(Self::ActivityTypeCreateSubOrganization)
                }
                "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V2" => {
                    Ok(Self::ActivityTypeCreateSubOrganizationV2)
                }
                "ACTIVITY_TYPE_UPDATE_ALLOWED_ORIGINS" => {
                    Ok(Self::ActivityTypeUpdateAllowedOrigins)
                }
                "ACTIVITY_TYPE_CREATE_PRIVATE_KEYS_V2" => Ok(Self::ActivityTypeCreatePrivateKeysV2),
                "ACTIVITY_TYPE_UPDATE_USER" => Ok(Self::ActivityTypeUpdateUser),
                "ACTIVITY_TYPE_UPDATE_POLICY" => Ok(Self::ActivityTypeUpdatePolicy),
                "ACTIVITY_TYPE_SET_PAYMENT_METHOD_V2" => Ok(Self::ActivityTypeSetPaymentMethodV2),
                "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V3" => {
                    Ok(Self::ActivityTypeCreateSubOrganizationV3)
                }
                "ACTIVITY_TYPE_CREATE_WALLET" => Ok(Self::ActivityTypeCreateWallet),
                "ACTIVITY_TYPE_CREATE_WALLET_ACCOUNTS" => {
                    Ok(Self::ActivityTypeCreateWalletAccounts)
                }
                "ACTIVITY_TYPE_INIT_USER_EMAIL_RECOVERY" => {
                    Ok(Self::ActivityTypeInitUserEmailRecovery)
                }
                "ACTIVITY_TYPE_RECOVER_USER" => Ok(Self::ActivityTypeRecoverUser),
                "ACTIVITY_TYPE_SET_ORGANIZATION_FEATURE" => {
                    Ok(Self::ActivityTypeSetOrganizationFeature)
                }
                "ACTIVITY_TYPE_REMOVE_ORGANIZATION_FEATURE" => {
                    Ok(Self::ActivityTypeRemoveOrganizationFeature)
                }
                "ACTIVITY_TYPE_SIGN_RAW_PAYLOAD_V2" => Ok(Self::ActivityTypeSignRawPayloadV2),
                "ACTIVITY_TYPE_SIGN_TRANSACTION_V2" => Ok(Self::ActivityTypeSignTransactionV2),
                "ACTIVITY_TYPE_EXPORT_PRIVATE_KEY" => Ok(Self::ActivityTypeExportPrivateKey),
                "ACTIVITY_TYPE_EXPORT_WALLET" => Ok(Self::ActivityTypeExportWallet),
                "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V4" => {
                    Ok(Self::ActivityTypeCreateSubOrganizationV4)
                }
                "ACTIVITY_TYPE_EMAIL_AUTH" => Ok(Self::ActivityTypeEmailAuth),
                "ACTIVITY_TYPE_EXPORT_WALLET_ACCOUNT" => Ok(Self::ActivityTypeExportWalletAccount),
                "ACTIVITY_TYPE_INIT_IMPORT_WALLET" => Ok(Self::ActivityTypeInitImportWallet),
                "ACTIVITY_TYPE_IMPORT_WALLET" => Ok(Self::ActivityTypeImportWallet),
                "ACTIVITY_TYPE_INIT_IMPORT_PRIVATE_KEY" => {
                    Ok(Self::ActivityTypeInitImportPrivateKey)
                }
                "ACTIVITY_TYPE_IMPORT_PRIVATE_KEY" => Ok(Self::ActivityTypeImportPrivateKey),
                "ACTIVITY_TYPE_CREATE_POLICIES" => Ok(Self::ActivityTypeCreatePolicies),
                "ACTIVITY_TYPE_SIGN_RAW_PAYLOADS" => Ok(Self::ActivityTypeSignRawPayloads),
                "ACTIVITY_TYPE_CREATE_READ_ONLY_SESSION" => {
                    Ok(Self::ActivityTypeCreateReadOnlySession)
                }
                "ACTIVITY_TYPE_CREATE_OAUTH_PROVIDERS" => {
                    Ok(Self::ActivityTypeCreateOauthProviders)
                }
                "ACTIVITY_TYPE_DELETE_OAUTH_PROVIDERS" => {
                    Ok(Self::ActivityTypeDeleteOauthProviders)
                }
                "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V5" => {
                    Ok(Self::ActivityTypeCreateSubOrganizationV5)
                }
                "ACTIVITY_TYPE_OAUTH" => Ok(Self::ActivityTypeOauth),
                "ACTIVITY_TYPE_CREATE_API_KEYS_V2" => Ok(Self::ActivityTypeCreateApiKeysV2),
                "ACTIVITY_TYPE_CREATE_READ_WRITE_SESSION" => {
                    Ok(Self::ActivityTypeCreateReadWriteSession)
                }
                "ACTIVITY_TYPE_EMAIL_AUTH_V2" => Ok(Self::ActivityTypeEmailAuthV2),
                "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V6" => {
                    Ok(Self::ActivityTypeCreateSubOrganizationV6)
                }
                "ACTIVITY_TYPE_DELETE_PRIVATE_KEYS" => Ok(Self::ActivityTypeDeletePrivateKeys),
                "ACTIVITY_TYPE_DELETE_WALLETS" => Ok(Self::ActivityTypeDeleteWallets),
                "ACTIVITY_TYPE_CREATE_READ_WRITE_SESSION_V2" => {
                    Ok(Self::ActivityTypeCreateReadWriteSessionV2)
                }
                "ACTIVITY_TYPE_DELETE_SUB_ORGANIZATION" => {
                    Ok(Self::ActivityTypeDeleteSubOrganization)
                }
                "ACTIVITY_TYPE_INIT_OTP_AUTH" => Ok(Self::ActivityTypeInitOtpAuth),
                "ACTIVITY_TYPE_OTP_AUTH" => Ok(Self::ActivityTypeOtpAuth),
                "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V7" => {
                    Ok(Self::ActivityTypeCreateSubOrganizationV7)
                }
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for ActivityType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for ActivityType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for ActivityType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///ActivityV1Address
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "address": {
    ///      "type": "string"
    ///    },
    ///    "format": {
    ///      "$ref": "#/components/schemas/AddressFormat"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ActivityV1Address {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub address: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub format: Option<AddressFormat>,
    }

    impl From<&ActivityV1Address> for ActivityV1Address {
        fn from(value: &ActivityV1Address) -> Self {
            value.clone()
        }
    }

    ///AddressFormat
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ADDRESS_FORMAT_UNCOMPRESSED",
    ///    "ADDRESS_FORMAT_COMPRESSED",
    ///    "ADDRESS_FORMAT_ETHEREUM",
    ///    "ADDRESS_FORMAT_SOLANA",
    ///    "ADDRESS_FORMAT_COSMOS",
    ///    "ADDRESS_FORMAT_TRON",
    ///    "ADDRESS_FORMAT_SUI",
    ///    "ADDRESS_FORMAT_APTOS",
    ///    "ADDRESS_FORMAT_BITCOIN_MAINNET_P2PKH",
    ///    "ADDRESS_FORMAT_BITCOIN_MAINNET_P2SH",
    ///    "ADDRESS_FORMAT_BITCOIN_MAINNET_P2WPKH",
    ///    "ADDRESS_FORMAT_BITCOIN_MAINNET_P2WSH",
    ///    "ADDRESS_FORMAT_BITCOIN_MAINNET_P2TR",
    ///    "ADDRESS_FORMAT_BITCOIN_TESTNET_P2PKH",
    ///    "ADDRESS_FORMAT_BITCOIN_TESTNET_P2SH",
    ///    "ADDRESS_FORMAT_BITCOIN_TESTNET_P2WPKH",
    ///    "ADDRESS_FORMAT_BITCOIN_TESTNET_P2WSH",
    ///    "ADDRESS_FORMAT_BITCOIN_TESTNET_P2TR",
    ///    "ADDRESS_FORMAT_BITCOIN_SIGNET_P2PKH",
    ///    "ADDRESS_FORMAT_BITCOIN_SIGNET_P2SH",
    ///    "ADDRESS_FORMAT_BITCOIN_SIGNET_P2WPKH",
    ///    "ADDRESS_FORMAT_BITCOIN_SIGNET_P2WSH",
    ///    "ADDRESS_FORMAT_BITCOIN_SIGNET_P2TR",
    ///    "ADDRESS_FORMAT_BITCOIN_REGTEST_P2PKH",
    ///    "ADDRESS_FORMAT_BITCOIN_REGTEST_P2SH",
    ///    "ADDRESS_FORMAT_BITCOIN_REGTEST_P2WPKH",
    ///    "ADDRESS_FORMAT_BITCOIN_REGTEST_P2WSH",
    ///    "ADDRESS_FORMAT_BITCOIN_REGTEST_P2TR",
    ///    "ADDRESS_FORMAT_SEI",
    ///    "ADDRESS_FORMAT_XLM",
    ///    "ADDRESS_FORMAT_DOGE_MAINNET",
    ///    "ADDRESS_FORMAT_DOGE_TESTNET",
    ///    "ADDRESS_FORMAT_TON_V3R2",
    ///    "ADDRESS_FORMAT_TON_V4R2"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum AddressFormat {
        #[serde(rename = "ADDRESS_FORMAT_UNCOMPRESSED")]
        AddressFormatUncompressed,
        #[serde(rename = "ADDRESS_FORMAT_COMPRESSED")]
        AddressFormatCompressed,
        #[serde(rename = "ADDRESS_FORMAT_ETHEREUM")]
        AddressFormatEthereum,
        #[serde(rename = "ADDRESS_FORMAT_SOLANA")]
        AddressFormatSolana,
        #[serde(rename = "ADDRESS_FORMAT_COSMOS")]
        AddressFormatCosmos,
        #[serde(rename = "ADDRESS_FORMAT_TRON")]
        AddressFormatTron,
        #[serde(rename = "ADDRESS_FORMAT_SUI")]
        AddressFormatSui,
        #[serde(rename = "ADDRESS_FORMAT_APTOS")]
        AddressFormatAptos,
        #[serde(rename = "ADDRESS_FORMAT_BITCOIN_MAINNET_P2PKH")]
        AddressFormatBitcoinMainnetP2pkh,
        #[serde(rename = "ADDRESS_FORMAT_BITCOIN_MAINNET_P2SH")]
        AddressFormatBitcoinMainnetP2sh,
        #[serde(rename = "ADDRESS_FORMAT_BITCOIN_MAINNET_P2WPKH")]
        AddressFormatBitcoinMainnetP2wpkh,
        #[serde(rename = "ADDRESS_FORMAT_BITCOIN_MAINNET_P2WSH")]
        AddressFormatBitcoinMainnetP2wsh,
        #[serde(rename = "ADDRESS_FORMAT_BITCOIN_MAINNET_P2TR")]
        AddressFormatBitcoinMainnetP2tr,
        #[serde(rename = "ADDRESS_FORMAT_BITCOIN_TESTNET_P2PKH")]
        AddressFormatBitcoinTestnetP2pkh,
        #[serde(rename = "ADDRESS_FORMAT_BITCOIN_TESTNET_P2SH")]
        AddressFormatBitcoinTestnetP2sh,
        #[serde(rename = "ADDRESS_FORMAT_BITCOIN_TESTNET_P2WPKH")]
        AddressFormatBitcoinTestnetP2wpkh,
        #[serde(rename = "ADDRESS_FORMAT_BITCOIN_TESTNET_P2WSH")]
        AddressFormatBitcoinTestnetP2wsh,
        #[serde(rename = "ADDRESS_FORMAT_BITCOIN_TESTNET_P2TR")]
        AddressFormatBitcoinTestnetP2tr,
        #[serde(rename = "ADDRESS_FORMAT_BITCOIN_SIGNET_P2PKH")]
        AddressFormatBitcoinSignetP2pkh,
        #[serde(rename = "ADDRESS_FORMAT_BITCOIN_SIGNET_P2SH")]
        AddressFormatBitcoinSignetP2sh,
        #[serde(rename = "ADDRESS_FORMAT_BITCOIN_SIGNET_P2WPKH")]
        AddressFormatBitcoinSignetP2wpkh,
        #[serde(rename = "ADDRESS_FORMAT_BITCOIN_SIGNET_P2WSH")]
        AddressFormatBitcoinSignetP2wsh,
        #[serde(rename = "ADDRESS_FORMAT_BITCOIN_SIGNET_P2TR")]
        AddressFormatBitcoinSignetP2tr,
        #[serde(rename = "ADDRESS_FORMAT_BITCOIN_REGTEST_P2PKH")]
        AddressFormatBitcoinRegtestP2pkh,
        #[serde(rename = "ADDRESS_FORMAT_BITCOIN_REGTEST_P2SH")]
        AddressFormatBitcoinRegtestP2sh,
        #[serde(rename = "ADDRESS_FORMAT_BITCOIN_REGTEST_P2WPKH")]
        AddressFormatBitcoinRegtestP2wpkh,
        #[serde(rename = "ADDRESS_FORMAT_BITCOIN_REGTEST_P2WSH")]
        AddressFormatBitcoinRegtestP2wsh,
        #[serde(rename = "ADDRESS_FORMAT_BITCOIN_REGTEST_P2TR")]
        AddressFormatBitcoinRegtestP2tr,
        #[serde(rename = "ADDRESS_FORMAT_SEI")]
        AddressFormatSei,
        #[serde(rename = "ADDRESS_FORMAT_XLM")]
        AddressFormatXlm,
        #[serde(rename = "ADDRESS_FORMAT_DOGE_MAINNET")]
        AddressFormatDogeMainnet,
        #[serde(rename = "ADDRESS_FORMAT_DOGE_TESTNET")]
        AddressFormatDogeTestnet,
        #[serde(rename = "ADDRESS_FORMAT_TON_V3R2")]
        AddressFormatTonV3r2,
        #[serde(rename = "ADDRESS_FORMAT_TON_V4R2")]
        AddressFormatTonV4r2,
    }

    impl From<&AddressFormat> for AddressFormat {
        fn from(value: &AddressFormat) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for AddressFormat {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::AddressFormatUncompressed => write!(f, "ADDRESS_FORMAT_UNCOMPRESSED"),
                Self::AddressFormatCompressed => write!(f, "ADDRESS_FORMAT_COMPRESSED"),
                Self::AddressFormatEthereum => write!(f, "ADDRESS_FORMAT_ETHEREUM"),
                Self::AddressFormatSolana => write!(f, "ADDRESS_FORMAT_SOLANA"),
                Self::AddressFormatCosmos => write!(f, "ADDRESS_FORMAT_COSMOS"),
                Self::AddressFormatTron => write!(f, "ADDRESS_FORMAT_TRON"),
                Self::AddressFormatSui => write!(f, "ADDRESS_FORMAT_SUI"),
                Self::AddressFormatAptos => write!(f, "ADDRESS_FORMAT_APTOS"),
                Self::AddressFormatBitcoinMainnetP2pkh => {
                    write!(f, "ADDRESS_FORMAT_BITCOIN_MAINNET_P2PKH")
                }
                Self::AddressFormatBitcoinMainnetP2sh => {
                    write!(f, "ADDRESS_FORMAT_BITCOIN_MAINNET_P2SH")
                }
                Self::AddressFormatBitcoinMainnetP2wpkh => {
                    write!(f, "ADDRESS_FORMAT_BITCOIN_MAINNET_P2WPKH")
                }
                Self::AddressFormatBitcoinMainnetP2wsh => {
                    write!(f, "ADDRESS_FORMAT_BITCOIN_MAINNET_P2WSH")
                }
                Self::AddressFormatBitcoinMainnetP2tr => {
                    write!(f, "ADDRESS_FORMAT_BITCOIN_MAINNET_P2TR")
                }
                Self::AddressFormatBitcoinTestnetP2pkh => {
                    write!(f, "ADDRESS_FORMAT_BITCOIN_TESTNET_P2PKH")
                }
                Self::AddressFormatBitcoinTestnetP2sh => {
                    write!(f, "ADDRESS_FORMAT_BITCOIN_TESTNET_P2SH")
                }
                Self::AddressFormatBitcoinTestnetP2wpkh => {
                    write!(f, "ADDRESS_FORMAT_BITCOIN_TESTNET_P2WPKH")
                }
                Self::AddressFormatBitcoinTestnetP2wsh => {
                    write!(f, "ADDRESS_FORMAT_BITCOIN_TESTNET_P2WSH")
                }
                Self::AddressFormatBitcoinTestnetP2tr => {
                    write!(f, "ADDRESS_FORMAT_BITCOIN_TESTNET_P2TR")
                }
                Self::AddressFormatBitcoinSignetP2pkh => {
                    write!(f, "ADDRESS_FORMAT_BITCOIN_SIGNET_P2PKH")
                }
                Self::AddressFormatBitcoinSignetP2sh => {
                    write!(f, "ADDRESS_FORMAT_BITCOIN_SIGNET_P2SH")
                }
                Self::AddressFormatBitcoinSignetP2wpkh => {
                    write!(f, "ADDRESS_FORMAT_BITCOIN_SIGNET_P2WPKH")
                }
                Self::AddressFormatBitcoinSignetP2wsh => {
                    write!(f, "ADDRESS_FORMAT_BITCOIN_SIGNET_P2WSH")
                }
                Self::AddressFormatBitcoinSignetP2tr => {
                    write!(f, "ADDRESS_FORMAT_BITCOIN_SIGNET_P2TR")
                }
                Self::AddressFormatBitcoinRegtestP2pkh => {
                    write!(f, "ADDRESS_FORMAT_BITCOIN_REGTEST_P2PKH")
                }
                Self::AddressFormatBitcoinRegtestP2sh => {
                    write!(f, "ADDRESS_FORMAT_BITCOIN_REGTEST_P2SH")
                }
                Self::AddressFormatBitcoinRegtestP2wpkh => {
                    write!(f, "ADDRESS_FORMAT_BITCOIN_REGTEST_P2WPKH")
                }
                Self::AddressFormatBitcoinRegtestP2wsh => {
                    write!(f, "ADDRESS_FORMAT_BITCOIN_REGTEST_P2WSH")
                }
                Self::AddressFormatBitcoinRegtestP2tr => {
                    write!(f, "ADDRESS_FORMAT_BITCOIN_REGTEST_P2TR")
                }
                Self::AddressFormatSei => write!(f, "ADDRESS_FORMAT_SEI"),
                Self::AddressFormatXlm => write!(f, "ADDRESS_FORMAT_XLM"),
                Self::AddressFormatDogeMainnet => write!(f, "ADDRESS_FORMAT_DOGE_MAINNET"),
                Self::AddressFormatDogeTestnet => write!(f, "ADDRESS_FORMAT_DOGE_TESTNET"),
                Self::AddressFormatTonV3r2 => write!(f, "ADDRESS_FORMAT_TON_V3R2"),
                Self::AddressFormatTonV4r2 => write!(f, "ADDRESS_FORMAT_TON_V4R2"),
            }
        }
    }

    impl std::str::FromStr for AddressFormat {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ADDRESS_FORMAT_UNCOMPRESSED" => Ok(Self::AddressFormatUncompressed),
                "ADDRESS_FORMAT_COMPRESSED" => Ok(Self::AddressFormatCompressed),
                "ADDRESS_FORMAT_ETHEREUM" => Ok(Self::AddressFormatEthereum),
                "ADDRESS_FORMAT_SOLANA" => Ok(Self::AddressFormatSolana),
                "ADDRESS_FORMAT_COSMOS" => Ok(Self::AddressFormatCosmos),
                "ADDRESS_FORMAT_TRON" => Ok(Self::AddressFormatTron),
                "ADDRESS_FORMAT_SUI" => Ok(Self::AddressFormatSui),
                "ADDRESS_FORMAT_APTOS" => Ok(Self::AddressFormatAptos),
                "ADDRESS_FORMAT_BITCOIN_MAINNET_P2PKH" => {
                    Ok(Self::AddressFormatBitcoinMainnetP2pkh)
                }
                "ADDRESS_FORMAT_BITCOIN_MAINNET_P2SH" => Ok(Self::AddressFormatBitcoinMainnetP2sh),
                "ADDRESS_FORMAT_BITCOIN_MAINNET_P2WPKH" => {
                    Ok(Self::AddressFormatBitcoinMainnetP2wpkh)
                }
                "ADDRESS_FORMAT_BITCOIN_MAINNET_P2WSH" => {
                    Ok(Self::AddressFormatBitcoinMainnetP2wsh)
                }
                "ADDRESS_FORMAT_BITCOIN_MAINNET_P2TR" => Ok(Self::AddressFormatBitcoinMainnetP2tr),
                "ADDRESS_FORMAT_BITCOIN_TESTNET_P2PKH" => {
                    Ok(Self::AddressFormatBitcoinTestnetP2pkh)
                }
                "ADDRESS_FORMAT_BITCOIN_TESTNET_P2SH" => Ok(Self::AddressFormatBitcoinTestnetP2sh),
                "ADDRESS_FORMAT_BITCOIN_TESTNET_P2WPKH" => {
                    Ok(Self::AddressFormatBitcoinTestnetP2wpkh)
                }
                "ADDRESS_FORMAT_BITCOIN_TESTNET_P2WSH" => {
                    Ok(Self::AddressFormatBitcoinTestnetP2wsh)
                }
                "ADDRESS_FORMAT_BITCOIN_TESTNET_P2TR" => Ok(Self::AddressFormatBitcoinTestnetP2tr),
                "ADDRESS_FORMAT_BITCOIN_SIGNET_P2PKH" => Ok(Self::AddressFormatBitcoinSignetP2pkh),
                "ADDRESS_FORMAT_BITCOIN_SIGNET_P2SH" => Ok(Self::AddressFormatBitcoinSignetP2sh),
                "ADDRESS_FORMAT_BITCOIN_SIGNET_P2WPKH" => {
                    Ok(Self::AddressFormatBitcoinSignetP2wpkh)
                }
                "ADDRESS_FORMAT_BITCOIN_SIGNET_P2WSH" => Ok(Self::AddressFormatBitcoinSignetP2wsh),
                "ADDRESS_FORMAT_BITCOIN_SIGNET_P2TR" => Ok(Self::AddressFormatBitcoinSignetP2tr),
                "ADDRESS_FORMAT_BITCOIN_REGTEST_P2PKH" => {
                    Ok(Self::AddressFormatBitcoinRegtestP2pkh)
                }
                "ADDRESS_FORMAT_BITCOIN_REGTEST_P2SH" => Ok(Self::AddressFormatBitcoinRegtestP2sh),
                "ADDRESS_FORMAT_BITCOIN_REGTEST_P2WPKH" => {
                    Ok(Self::AddressFormatBitcoinRegtestP2wpkh)
                }
                "ADDRESS_FORMAT_BITCOIN_REGTEST_P2WSH" => {
                    Ok(Self::AddressFormatBitcoinRegtestP2wsh)
                }
                "ADDRESS_FORMAT_BITCOIN_REGTEST_P2TR" => Ok(Self::AddressFormatBitcoinRegtestP2tr),
                "ADDRESS_FORMAT_SEI" => Ok(Self::AddressFormatSei),
                "ADDRESS_FORMAT_XLM" => Ok(Self::AddressFormatXlm),
                "ADDRESS_FORMAT_DOGE_MAINNET" => Ok(Self::AddressFormatDogeMainnet),
                "ADDRESS_FORMAT_DOGE_TESTNET" => Ok(Self::AddressFormatDogeTestnet),
                "ADDRESS_FORMAT_TON_V3R2" => Ok(Self::AddressFormatTonV3r2),
                "ADDRESS_FORMAT_TON_V4R2" => Ok(Self::AddressFormatTonV4r2),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for AddressFormat {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for AddressFormat {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for AddressFormat {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///Any
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "@type": {
    ///      "type": "string"
    ///    }
    ///  },
    ///  "additionalProperties": {}
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct Any {
        #[serde(rename = "@type", default, skip_serializing_if = "Option::is_none")]
        pub type_: Option<String>,
    }

    impl From<&Any> for Any {
        fn from(value: &Any) -> Self {
            value.clone()
        }
    }

    ///ApiKey
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "apiKeyId",
    ///    "apiKeyName",
    ///    "createdAt",
    ///    "credential",
    ///    "updatedAt"
    ///  ],
    ///  "properties": {
    ///    "apiKeyId": {
    ///      "description": "Unique identifier for a given API Key.",
    ///      "type": "string"
    ///    },
    ///    "apiKeyName": {
    ///      "description": "Human-readable name for an API Key.",
    ///      "type": "string"
    ///    },
    ///    "createdAt": {
    ///      "$ref": "#/components/schemas/external.data.v1.Timestamp"
    ///    },
    ///    "credential": {
    ///      "$ref": "#/components/schemas/external.data.v1.Credential"
    ///    },
    ///    "expirationSeconds": {
    ///      "description": "Optional window (in seconds) indicating how long
    /// the API Key should last.",
    ///      "type": "string",
    ///      "format": "uint64"
    ///    },
    ///    "updatedAt": {
    ///      "$ref": "#/components/schemas/external.data.v1.Timestamp"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ApiKey {
        ///Unique identifier for a given API Key.
        #[serde(rename = "apiKeyId")]
        pub api_key_id: String,
        ///Human-readable name for an API Key.
        #[serde(rename = "apiKeyName")]
        pub api_key_name: String,
        #[serde(rename = "createdAt")]
        pub created_at: ExternalDataV1Timestamp,
        pub credential: ExternalDataV1Credential,
        ///Optional window (in seconds) indicating how long the API Key should
        /// last.
        #[serde(
            rename = "expirationSeconds",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub expiration_seconds: Option<String>,
        #[serde(rename = "updatedAt")]
        pub updated_at: ExternalDataV1Timestamp,
    }

    impl From<&ApiKey> for ApiKey {
        fn from(value: &ApiKey) -> Self {
            value.clone()
        }
    }

    ///ApiKeyCurve
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "API_KEY_CURVE_P256",
    ///    "API_KEY_CURVE_SECP256K1",
    ///    "API_KEY_CURVE_ED25519"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum ApiKeyCurve {
        #[serde(rename = "API_KEY_CURVE_P256")]
        ApiKeyCurveP256,
        #[serde(rename = "API_KEY_CURVE_SECP256K1")]
        ApiKeyCurveSecp256k1,
        #[serde(rename = "API_KEY_CURVE_ED25519")]
        ApiKeyCurveEd25519,
    }

    impl From<&ApiKeyCurve> for ApiKeyCurve {
        fn from(value: &ApiKeyCurve) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for ApiKeyCurve {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ApiKeyCurveP256 => write!(f, "API_KEY_CURVE_P256"),
                Self::ApiKeyCurveSecp256k1 => write!(f, "API_KEY_CURVE_SECP256K1"),
                Self::ApiKeyCurveEd25519 => write!(f, "API_KEY_CURVE_ED25519"),
            }
        }
    }

    impl std::str::FromStr for ApiKeyCurve {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "API_KEY_CURVE_P256" => Ok(Self::ApiKeyCurveP256),
                "API_KEY_CURVE_SECP256K1" => Ok(Self::ApiKeyCurveSecp256k1),
                "API_KEY_CURVE_ED25519" => Ok(Self::ApiKeyCurveEd25519),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for ApiKeyCurve {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for ApiKeyCurve {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for ApiKeyCurve {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///ApiKeyParams
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "apiKeyName",
    ///    "publicKey"
    ///  ],
    ///  "properties": {
    ///    "apiKeyName": {
    ///      "description": "Human-readable name for an API Key.",
    ///      "type": "string"
    ///    },
    ///    "expirationSeconds": {
    ///      "description": "Optional window (in seconds) indicating how long
    /// the API Key should last.",
    ///      "type": "string"
    ///    },
    ///    "publicKey": {
    ///      "description": "The public component of a cryptographic key pair
    /// used to sign messages and transactions.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ApiKeyParams {
        ///Human-readable name for an API Key.
        #[serde(rename = "apiKeyName")]
        pub api_key_name: String,
        ///Optional window (in seconds) indicating how long the API Key should
        /// last.
        #[serde(
            rename = "expirationSeconds",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub expiration_seconds: Option<String>,
        ///The public component of a cryptographic key pair used to sign
        /// messages and transactions.
        #[serde(rename = "publicKey")]
        pub public_key: String,
    }

    impl From<&ApiKeyParams> for ApiKeyParams {
        fn from(value: &ApiKeyParams) -> Self {
            value.clone()
        }
    }

    ///ApiKeyParamsV2
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "apiKeyName",
    ///    "curveType",
    ///    "publicKey"
    ///  ],
    ///  "properties": {
    ///    "apiKeyName": {
    ///      "description": "Human-readable name for an API Key.",
    ///      "type": "string"
    ///    },
    ///    "curveType": {
    ///      "$ref": "#/components/schemas/ApiKeyCurve"
    ///    },
    ///    "expirationSeconds": {
    ///      "description": "Optional window (in seconds) indicating how long
    /// the API Key should last.",
    ///      "type": "string"
    ///    },
    ///    "publicKey": {
    ///      "description": "The public component of a cryptographic key pair
    /// used to sign messages and transactions.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ApiKeyParamsV2 {
        ///Human-readable name for an API Key.
        #[serde(rename = "apiKeyName")]
        pub api_key_name: String,
        #[serde(rename = "curveType")]
        pub curve_type: ApiKeyCurve,
        ///Optional window (in seconds) indicating how long the API Key should
        /// last.
        #[serde(
            rename = "expirationSeconds",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub expiration_seconds: Option<String>,
        ///The public component of a cryptographic key pair used to sign
        /// messages and transactions.
        #[serde(rename = "publicKey")]
        pub public_key: String,
    }

    impl From<&ApiKeyParamsV2> for ApiKeyParamsV2 {
        fn from(value: &ApiKeyParamsV2) -> Self {
            value.clone()
        }
    }

    ///ApiOnlyUserParams
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "apiKeys",
    ///    "userName",
    ///    "userTags"
    ///  ],
    ///  "properties": {
    ///    "apiKeys": {
    ///      "description": "A list of API Key parameters.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/ApiKeyParams"
    ///      }
    ///    },
    ///    "userEmail": {
    ///      "description": "The email address for this API-only User
    /// (optional).",
    ///      "type": "string"
    ///    },
    ///    "userName": {
    ///      "description": "The name of the new API-only User.",
    ///      "type": "string"
    ///    },
    ///    "userTags": {
    ///      "description": "A list of tags assigned to the new API-only User.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ApiOnlyUserParams {
        ///A list of API Key parameters.
        #[serde(rename = "apiKeys")]
        pub api_keys: Vec<ApiKeyParams>,
        ///The email address for this API-only User (optional).
        #[serde(rename = "userEmail", default, skip_serializing_if = "Option::is_none")]
        pub user_email: Option<String>,
        ///The name of the new API-only User.
        #[serde(rename = "userName")]
        pub user_name: String,
        ///A list of tags assigned to the new API-only User.
        #[serde(rename = "userTags")]
        pub user_tags: Vec<String>,
    }

    impl From<&ApiOnlyUserParams> for ApiOnlyUserParams {
        fn from(value: &ApiOnlyUserParams) -> Self {
            value.clone()
        }
    }

    ///ApproveActivityIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "fingerprint"
    ///  ],
    ///  "properties": {
    ///    "fingerprint": {
    ///      "description": "An artifact verifying a User's action.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ApproveActivityIntent {
        ///An artifact verifying a User's action.
        pub fingerprint: String,
    }

    impl From<&ApproveActivityIntent> for ApproveActivityIntent {
        fn from(value: &ApproveActivityIntent) -> Self {
            value.clone()
        }
    }

    ///ApproveActivityRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/ApproveActivityIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_APPROVE_ACTIVITY"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ApproveActivityRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: ApproveActivityIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: ApproveActivityRequestType,
    }

    impl From<&ApproveActivityRequest> for ApproveActivityRequest {
        fn from(value: &ApproveActivityRequest) -> Self {
            value.clone()
        }
    }

    ///ApproveActivityRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_APPROVE_ACTIVITY"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum ApproveActivityRequestType {
        #[serde(rename = "ACTIVITY_TYPE_APPROVE_ACTIVITY")]
        ActivityTypeApproveActivity,
    }

    impl From<&ApproveActivityRequestType> for ApproveActivityRequestType {
        fn from(value: &ApproveActivityRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for ApproveActivityRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeApproveActivity => write!(f, "ACTIVITY_TYPE_APPROVE_ACTIVITY"),
            }
        }
    }

    impl std::str::FromStr for ApproveActivityRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_APPROVE_ACTIVITY" => Ok(Self::ActivityTypeApproveActivity),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for ApproveActivityRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for ApproveActivityRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for ApproveActivityRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///Attestation
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "attestationObject",
    ///    "clientDataJson",
    ///    "credentialId",
    ///    "transports"
    ///  ],
    ///  "properties": {
    ///    "attestationObject": {
    ///      "description": "A base64 url encoded payload containing authenticator data and any attestation the webauthn provider chooses.",
    ///      "type": "string"
    ///    },
    ///    "clientDataJson": {
    ///      "description": "A base64 url encoded payload containing metadata
    /// about the signing context and the challenge.",
    ///      "type": "string"
    ///    },
    ///    "credentialId": {
    ///      "description": "The cbor encoded then base64 url encoded id of the
    /// credential.",
    ///      "type": "string"
    ///    },
    ///    "transports": {
    ///      "description": "The type of authenticator transports.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/AuthenticatorTransport"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct Attestation {
        ///A base64 url encoded payload containing authenticator data and any
        /// attestation the webauthn provider chooses.
        #[serde(rename = "attestationObject")]
        pub attestation_object: String,
        ///A base64 url encoded payload containing metadata about the signing
        /// context and the challenge.
        #[serde(rename = "clientDataJson")]
        pub client_data_json: String,
        ///The cbor encoded then base64 url encoded id of the credential.
        #[serde(rename = "credentialId")]
        pub credential_id: String,
        ///The type of authenticator transports.
        pub transports: Vec<AuthenticatorTransport>,
    }

    impl From<&Attestation> for Attestation {
        fn from(value: &Attestation) -> Self {
            value.clone()
        }
    }

    ///Authenticator
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "aaguid",
    ///    "attestationType",
    ///    "authenticatorId",
    ///    "authenticatorName",
    ///    "createdAt",
    ///    "credential",
    ///    "credentialId",
    ///    "model",
    ///    "transports",
    ///    "updatedAt"
    ///  ],
    ///  "properties": {
    ///    "aaguid": {
    ///      "description": "Identifier indicating the type of the Security
    /// Key.",
    ///      "type": "string"
    ///    },
    ///    "attestationType": {
    ///      "type": "string"
    ///    },
    ///    "authenticatorId": {
    ///      "description": "Unique identifier for a given Authenticator.",
    ///      "type": "string"
    ///    },
    ///    "authenticatorName": {
    ///      "description": "Human-readable name for an Authenticator.",
    ///      "type": "string"
    ///    },
    ///    "createdAt": {
    ///      "$ref": "#/components/schemas/external.data.v1.Timestamp"
    ///    },
    ///    "credential": {
    ///      "$ref": "#/components/schemas/external.data.v1.Credential"
    ///    },
    ///    "credentialId": {
    ///      "description": "Unique identifier for a WebAuthn credential.",
    ///      "type": "string"
    ///    },
    ///    "model": {
    ///      "description": "The type of Authenticator device.",
    ///      "type": "string"
    ///    },
    ///    "transports": {
    ///      "description": "Types of transports that may be used by an
    /// Authenticator (e.g., USB, NFC, BLE).",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/AuthenticatorTransport"
    ///      }
    ///    },
    ///    "updatedAt": {
    ///      "$ref": "#/components/schemas/external.data.v1.Timestamp"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct Authenticator {
        ///Identifier indicating the type of the Security Key.
        pub aaguid: String,
        #[serde(rename = "attestationType")]
        pub attestation_type: String,
        ///Unique identifier for a given Authenticator.
        #[serde(rename = "authenticatorId")]
        pub authenticator_id: String,
        ///Human-readable name for an Authenticator.
        #[serde(rename = "authenticatorName")]
        pub authenticator_name: String,
        #[serde(rename = "createdAt")]
        pub created_at: ExternalDataV1Timestamp,
        pub credential: ExternalDataV1Credential,
        ///Unique identifier for a WebAuthn credential.
        #[serde(rename = "credentialId")]
        pub credential_id: String,
        ///The type of Authenticator device.
        pub model: String,
        ///Types of transports that may be used by an Authenticator (e.g., USB,
        /// NFC, BLE).
        pub transports: Vec<AuthenticatorTransport>,
        #[serde(rename = "updatedAt")]
        pub updated_at: ExternalDataV1Timestamp,
    }

    impl From<&Authenticator> for Authenticator {
        fn from(value: &Authenticator) -> Self {
            value.clone()
        }
    }

    ///AuthenticatorAttestationResponse
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "attestationObject",
    ///    "clientDataJson"
    ///  ],
    ///  "properties": {
    ///    "attestationObject": {
    ///      "type": "string"
    ///    },
    ///    "authenticatorAttachment": {
    ///      "type": [
    ///        "string",
    ///        "null"
    ///      ],
    ///      "enum": [
    ///        "cross-platform",
    ///        "platform"
    ///      ]
    ///    },
    ///    "clientDataJson": {
    ///      "type": "string"
    ///    },
    ///    "transports": {
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/AuthenticatorTransport"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct AuthenticatorAttestationResponse {
        #[serde(rename = "attestationObject")]
        pub attestation_object: String,
        #[serde(
            rename = "authenticatorAttachment",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub authenticator_attachment:
            Option<AuthenticatorAttestationResponseAuthenticatorAttachment>,
        #[serde(rename = "clientDataJson")]
        pub client_data_json: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub transports: Vec<AuthenticatorTransport>,
    }

    impl From<&AuthenticatorAttestationResponse> for AuthenticatorAttestationResponse {
        fn from(value: &AuthenticatorAttestationResponse) -> Self {
            value.clone()
        }
    }

    ///AuthenticatorAttestationResponseAuthenticatorAttachment
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "cross-platform",
    ///    "platform"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum AuthenticatorAttestationResponseAuthenticatorAttachment {
        #[serde(rename = "cross-platform")]
        CrossPlatform,
        #[serde(rename = "platform")]
        Platform,
    }

    impl From<&AuthenticatorAttestationResponseAuthenticatorAttachment>
        for AuthenticatorAttestationResponseAuthenticatorAttachment
    {
        fn from(value: &AuthenticatorAttestationResponseAuthenticatorAttachment) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for AuthenticatorAttestationResponseAuthenticatorAttachment {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::CrossPlatform => write!(f, "cross-platform"),
                Self::Platform => write!(f, "platform"),
            }
        }
    }

    impl std::str::FromStr for AuthenticatorAttestationResponseAuthenticatorAttachment {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "cross-platform" => Ok(Self::CrossPlatform),
                "platform" => Ok(Self::Platform),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for AuthenticatorAttestationResponseAuthenticatorAttachment {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for AuthenticatorAttestationResponseAuthenticatorAttachment {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for AuthenticatorAttestationResponseAuthenticatorAttachment {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///AuthenticatorParams
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "attestation",
    ///    "authenticatorName",
    ///    "challenge",
    ///    "userId"
    ///  ],
    ///  "properties": {
    ///    "attestation": {
    ///      "$ref": "#/components/schemas/PublicKeyCredentialWithAttestation"
    ///    },
    ///    "authenticatorName": {
    ///      "description": "Human-readable name for an Authenticator.",
    ///      "type": "string"
    ///    },
    ///    "challenge": {
    ///      "description": "Challenge presented for authentication purposes.",
    ///      "type": "string"
    ///    },
    ///    "userId": {
    ///      "description": "Unique identifier for a given User.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct AuthenticatorParams {
        pub attestation: PublicKeyCredentialWithAttestation,
        ///Human-readable name for an Authenticator.
        #[serde(rename = "authenticatorName")]
        pub authenticator_name: String,
        ///Challenge presented for authentication purposes.
        pub challenge: String,
        ///Unique identifier for a given User.
        #[serde(rename = "userId")]
        pub user_id: String,
    }

    impl From<&AuthenticatorParams> for AuthenticatorParams {
        fn from(value: &AuthenticatorParams) -> Self {
            value.clone()
        }
    }

    ///AuthenticatorParamsV2
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "attestation",
    ///    "authenticatorName",
    ///    "challenge"
    ///  ],
    ///  "properties": {
    ///    "attestation": {
    ///      "$ref": "#/components/schemas/Attestation"
    ///    },
    ///    "authenticatorName": {
    ///      "description": "Human-readable name for an Authenticator.",
    ///      "type": "string"
    ///    },
    ///    "challenge": {
    ///      "description": "Challenge presented for authentication purposes.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct AuthenticatorParamsV2 {
        pub attestation: Attestation,
        ///Human-readable name for an Authenticator.
        #[serde(rename = "authenticatorName")]
        pub authenticator_name: String,
        ///Challenge presented for authentication purposes.
        pub challenge: String,
    }

    impl From<&AuthenticatorParamsV2> for AuthenticatorParamsV2 {
        fn from(value: &AuthenticatorParamsV2) -> Self {
            value.clone()
        }
    }

    ///AuthenticatorTransport
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "AUTHENTICATOR_TRANSPORT_BLE",
    ///    "AUTHENTICATOR_TRANSPORT_INTERNAL",
    ///    "AUTHENTICATOR_TRANSPORT_NFC",
    ///    "AUTHENTICATOR_TRANSPORT_USB",
    ///    "AUTHENTICATOR_TRANSPORT_HYBRID"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum AuthenticatorTransport {
        #[serde(rename = "AUTHENTICATOR_TRANSPORT_BLE")]
        AuthenticatorTransportBle,
        #[serde(rename = "AUTHENTICATOR_TRANSPORT_INTERNAL")]
        AuthenticatorTransportInternal,
        #[serde(rename = "AUTHENTICATOR_TRANSPORT_NFC")]
        AuthenticatorTransportNfc,
        #[serde(rename = "AUTHENTICATOR_TRANSPORT_USB")]
        AuthenticatorTransportUsb,
        #[serde(rename = "AUTHENTICATOR_TRANSPORT_HYBRID")]
        AuthenticatorTransportHybrid,
    }

    impl From<&AuthenticatorTransport> for AuthenticatorTransport {
        fn from(value: &AuthenticatorTransport) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for AuthenticatorTransport {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::AuthenticatorTransportBle => write!(f, "AUTHENTICATOR_TRANSPORT_BLE"),
                Self::AuthenticatorTransportInternal => {
                    write!(f, "AUTHENTICATOR_TRANSPORT_INTERNAL")
                }
                Self::AuthenticatorTransportNfc => write!(f, "AUTHENTICATOR_TRANSPORT_NFC"),
                Self::AuthenticatorTransportUsb => write!(f, "AUTHENTICATOR_TRANSPORT_USB"),
                Self::AuthenticatorTransportHybrid => write!(f, "AUTHENTICATOR_TRANSPORT_HYBRID"),
            }
        }
    }

    impl std::str::FromStr for AuthenticatorTransport {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "AUTHENTICATOR_TRANSPORT_BLE" => Ok(Self::AuthenticatorTransportBle),
                "AUTHENTICATOR_TRANSPORT_INTERNAL" => Ok(Self::AuthenticatorTransportInternal),
                "AUTHENTICATOR_TRANSPORT_NFC" => Ok(Self::AuthenticatorTransportNfc),
                "AUTHENTICATOR_TRANSPORT_USB" => Ok(Self::AuthenticatorTransportUsb),
                "AUTHENTICATOR_TRANSPORT_HYBRID" => Ok(Self::AuthenticatorTransportHybrid),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for AuthenticatorTransport {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for AuthenticatorTransport {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for AuthenticatorTransport {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///Config
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "features": {
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/Feature"
    ///      }
    ///    },
    ///    "quorum": {
    ///      "$ref": "#/components/schemas/external.data.v1.Quorum"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct Config {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub features: Vec<Feature>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub quorum: Option<ExternalDataV1Quorum>,
    }

    impl From<&Config> for Config {
        fn from(value: &Config) -> Self {
            value.clone()
        }
    }

    ///CreateApiKeysIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "apiKeys",
    ///    "userId"
    ///  ],
    ///  "properties": {
    ///    "apiKeys": {
    ///      "description": "A list of API Keys.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/ApiKeyParams"
    ///      }
    ///    },
    ///    "userId": {
    ///      "description": "Unique identifier for a given User.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateApiKeysIntent {
        ///A list of API Keys.
        #[serde(rename = "apiKeys")]
        pub api_keys: Vec<ApiKeyParams>,
        ///Unique identifier for a given User.
        #[serde(rename = "userId")]
        pub user_id: String,
    }

    impl From<&CreateApiKeysIntent> for CreateApiKeysIntent {
        fn from(value: &CreateApiKeysIntent) -> Self {
            value.clone()
        }
    }

    ///CreateApiKeysIntentV2
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "apiKeys",
    ///    "userId"
    ///  ],
    ///  "properties": {
    ///    "apiKeys": {
    ///      "description": "A list of API Keys.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/ApiKeyParamsV2"
    ///      }
    ///    },
    ///    "userId": {
    ///      "description": "Unique identifier for a given User.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateApiKeysIntentV2 {
        ///A list of API Keys.
        #[serde(rename = "apiKeys")]
        pub api_keys: Vec<ApiKeyParamsV2>,
        ///Unique identifier for a given User.
        #[serde(rename = "userId")]
        pub user_id: String,
    }

    impl From<&CreateApiKeysIntentV2> for CreateApiKeysIntentV2 {
        fn from(value: &CreateApiKeysIntentV2) -> Self {
            value.clone()
        }
    }

    ///CreateApiKeysRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/CreateApiKeysIntentV2"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_CREATE_API_KEYS_V2"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateApiKeysRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: CreateApiKeysIntentV2,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: CreateApiKeysRequestType,
    }

    impl From<&CreateApiKeysRequest> for CreateApiKeysRequest {
        fn from(value: &CreateApiKeysRequest) -> Self {
            value.clone()
        }
    }

    ///CreateApiKeysRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_CREATE_API_KEYS_V2"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum CreateApiKeysRequestType {
        #[serde(rename = "ACTIVITY_TYPE_CREATE_API_KEYS_V2")]
        ActivityTypeCreateApiKeysV2,
    }

    impl From<&CreateApiKeysRequestType> for CreateApiKeysRequestType {
        fn from(value: &CreateApiKeysRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for CreateApiKeysRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeCreateApiKeysV2 => write!(f, "ACTIVITY_TYPE_CREATE_API_KEYS_V2"),
            }
        }
    }

    impl std::str::FromStr for CreateApiKeysRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_CREATE_API_KEYS_V2" => Ok(Self::ActivityTypeCreateApiKeysV2),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for CreateApiKeysRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for CreateApiKeysRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for CreateApiKeysRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///CreateApiKeysResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "apiKeyIds"
    ///  ],
    ///  "properties": {
    ///    "apiKeyIds": {
    ///      "description": "A list of API Key IDs.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateApiKeysResult {
        ///A list of API Key IDs.
        #[serde(rename = "apiKeyIds")]
        pub api_key_ids: Vec<String>,
    }

    impl From<&CreateApiKeysResult> for CreateApiKeysResult {
        fn from(value: &CreateApiKeysResult) -> Self {
            value.clone()
        }
    }

    ///CreateApiOnlyUsersIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "apiOnlyUsers"
    ///  ],
    ///  "properties": {
    ///    "apiOnlyUsers": {
    ///      "description": "A list of API-only Users to create.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/ApiOnlyUserParams"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateApiOnlyUsersIntent {
        ///A list of API-only Users to create.
        #[serde(rename = "apiOnlyUsers")]
        pub api_only_users: Vec<ApiOnlyUserParams>,
    }

    impl From<&CreateApiOnlyUsersIntent> for CreateApiOnlyUsersIntent {
        fn from(value: &CreateApiOnlyUsersIntent) -> Self {
            value.clone()
        }
    }

    ///CreateApiOnlyUsersResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "userIds"
    ///  ],
    ///  "properties": {
    ///    "userIds": {
    ///      "description": "A list of API-only User IDs.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateApiOnlyUsersResult {
        ///A list of API-only User IDs.
        #[serde(rename = "userIds")]
        pub user_ids: Vec<String>,
    }

    impl From<&CreateApiOnlyUsersResult> for CreateApiOnlyUsersResult {
        fn from(value: &CreateApiOnlyUsersResult) -> Self {
            value.clone()
        }
    }

    ///CreateAuthenticatorsIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "authenticators",
    ///    "userId"
    ///  ],
    ///  "properties": {
    ///    "authenticators": {
    ///      "description": "A list of Authenticators.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/AuthenticatorParams"
    ///      }
    ///    },
    ///    "userId": {
    ///      "description": "Unique identifier for a given User.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateAuthenticatorsIntent {
        ///A list of Authenticators.
        pub authenticators: Vec<AuthenticatorParams>,
        ///Unique identifier for a given User.
        #[serde(rename = "userId")]
        pub user_id: String,
    }

    impl From<&CreateAuthenticatorsIntent> for CreateAuthenticatorsIntent {
        fn from(value: &CreateAuthenticatorsIntent) -> Self {
            value.clone()
        }
    }

    ///CreateAuthenticatorsIntentV2
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "authenticators",
    ///    "userId"
    ///  ],
    ///  "properties": {
    ///    "authenticators": {
    ///      "description": "A list of Authenticators.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/AuthenticatorParamsV2"
    ///      }
    ///    },
    ///    "userId": {
    ///      "description": "Unique identifier for a given User.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateAuthenticatorsIntentV2 {
        ///A list of Authenticators.
        pub authenticators: Vec<AuthenticatorParamsV2>,
        ///Unique identifier for a given User.
        #[serde(rename = "userId")]
        pub user_id: String,
    }

    impl From<&CreateAuthenticatorsIntentV2> for CreateAuthenticatorsIntentV2 {
        fn from(value: &CreateAuthenticatorsIntentV2) -> Self {
            value.clone()
        }
    }

    ///CreateAuthenticatorsRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/CreateAuthenticatorsIntentV2"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_CREATE_AUTHENTICATORS_V2"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateAuthenticatorsRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: CreateAuthenticatorsIntentV2,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: CreateAuthenticatorsRequestType,
    }

    impl From<&CreateAuthenticatorsRequest> for CreateAuthenticatorsRequest {
        fn from(value: &CreateAuthenticatorsRequest) -> Self {
            value.clone()
        }
    }

    ///CreateAuthenticatorsRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_CREATE_AUTHENTICATORS_V2"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum CreateAuthenticatorsRequestType {
        #[serde(rename = "ACTIVITY_TYPE_CREATE_AUTHENTICATORS_V2")]
        ActivityTypeCreateAuthenticatorsV2,
    }

    impl From<&CreateAuthenticatorsRequestType> for CreateAuthenticatorsRequestType {
        fn from(value: &CreateAuthenticatorsRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for CreateAuthenticatorsRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeCreateAuthenticatorsV2 => {
                    write!(f, "ACTIVITY_TYPE_CREATE_AUTHENTICATORS_V2")
                }
            }
        }
    }

    impl std::str::FromStr for CreateAuthenticatorsRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_CREATE_AUTHENTICATORS_V2" => {
                    Ok(Self::ActivityTypeCreateAuthenticatorsV2)
                }
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for CreateAuthenticatorsRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for CreateAuthenticatorsRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for CreateAuthenticatorsRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///CreateAuthenticatorsResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "authenticatorIds"
    ///  ],
    ///  "properties": {
    ///    "authenticatorIds": {
    ///      "description": "A list of Authenticator IDs.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateAuthenticatorsResult {
        ///A list of Authenticator IDs.
        #[serde(rename = "authenticatorIds")]
        pub authenticator_ids: Vec<String>,
    }

    impl From<&CreateAuthenticatorsResult> for CreateAuthenticatorsResult {
        fn from(value: &CreateAuthenticatorsResult) -> Self {
            value.clone()
        }
    }

    ///CreateInvitationsIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "invitations"
    ///  ],
    ///  "properties": {
    ///    "invitations": {
    ///      "description": "A list of Invitations.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/InvitationParams"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateInvitationsIntent {
        ///A list of Invitations.
        pub invitations: Vec<InvitationParams>,
    }

    impl From<&CreateInvitationsIntent> for CreateInvitationsIntent {
        fn from(value: &CreateInvitationsIntent) -> Self {
            value.clone()
        }
    }

    ///CreateInvitationsRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/CreateInvitationsIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_CREATE_INVITATIONS"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateInvitationsRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: CreateInvitationsIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: CreateInvitationsRequestType,
    }

    impl From<&CreateInvitationsRequest> for CreateInvitationsRequest {
        fn from(value: &CreateInvitationsRequest) -> Self {
            value.clone()
        }
    }

    ///CreateInvitationsRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_CREATE_INVITATIONS"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum CreateInvitationsRequestType {
        #[serde(rename = "ACTIVITY_TYPE_CREATE_INVITATIONS")]
        ActivityTypeCreateInvitations,
    }

    impl From<&CreateInvitationsRequestType> for CreateInvitationsRequestType {
        fn from(value: &CreateInvitationsRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for CreateInvitationsRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeCreateInvitations => {
                    write!(f, "ACTIVITY_TYPE_CREATE_INVITATIONS")
                }
            }
        }
    }

    impl std::str::FromStr for CreateInvitationsRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_CREATE_INVITATIONS" => Ok(Self::ActivityTypeCreateInvitations),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for CreateInvitationsRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for CreateInvitationsRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for CreateInvitationsRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///CreateInvitationsResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "invitationIds"
    ///  ],
    ///  "properties": {
    ///    "invitationIds": {
    ///      "description": "A list of Invitation IDs",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateInvitationsResult {
        ///A list of Invitation IDs
        #[serde(rename = "invitationIds")]
        pub invitation_ids: Vec<String>,
    }

    impl From<&CreateInvitationsResult> for CreateInvitationsResult {
        fn from(value: &CreateInvitationsResult) -> Self {
            value.clone()
        }
    }

    ///CreateOauthProvidersIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "oauthProviders",
    ///    "userId"
    ///  ],
    ///  "properties": {
    ///    "oauthProviders": {
    ///      "description": "A list of Oauth providers.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/OauthProviderParams"
    ///      }
    ///    },
    ///    "userId": {
    ///      "description": "The ID of the User to add an Oauth provider to",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateOauthProvidersIntent {
        ///A list of Oauth providers.
        #[serde(rename = "oauthProviders")]
        pub oauth_providers: Vec<OauthProviderParams>,
        ///The ID of the User to add an Oauth provider to
        #[serde(rename = "userId")]
        pub user_id: String,
    }

    impl From<&CreateOauthProvidersIntent> for CreateOauthProvidersIntent {
        fn from(value: &CreateOauthProvidersIntent) -> Self {
            value.clone()
        }
    }

    ///CreateOauthProvidersRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/CreateOauthProvidersIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_CREATE_OAUTH_PROVIDERS"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateOauthProvidersRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: CreateOauthProvidersIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: CreateOauthProvidersRequestType,
    }

    impl From<&CreateOauthProvidersRequest> for CreateOauthProvidersRequest {
        fn from(value: &CreateOauthProvidersRequest) -> Self {
            value.clone()
        }
    }

    ///CreateOauthProvidersRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_CREATE_OAUTH_PROVIDERS"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum CreateOauthProvidersRequestType {
        #[serde(rename = "ACTIVITY_TYPE_CREATE_OAUTH_PROVIDERS")]
        ActivityTypeCreateOauthProviders,
    }

    impl From<&CreateOauthProvidersRequestType> for CreateOauthProvidersRequestType {
        fn from(value: &CreateOauthProvidersRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for CreateOauthProvidersRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeCreateOauthProviders => {
                    write!(f, "ACTIVITY_TYPE_CREATE_OAUTH_PROVIDERS")
                }
            }
        }
    }

    impl std::str::FromStr for CreateOauthProvidersRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_CREATE_OAUTH_PROVIDERS" => {
                    Ok(Self::ActivityTypeCreateOauthProviders)
                }
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for CreateOauthProvidersRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for CreateOauthProvidersRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for CreateOauthProvidersRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///CreateOauthProvidersResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "providerIds"
    ///  ],
    ///  "properties": {
    ///    "providerIds": {
    ///      "description": "A list of unique identifiers for Oauth Providers",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateOauthProvidersResult {
        ///A list of unique identifiers for Oauth Providers
        #[serde(rename = "providerIds")]
        pub provider_ids: Vec<String>,
    }

    impl From<&CreateOauthProvidersResult> for CreateOauthProvidersResult {
        fn from(value: &CreateOauthProvidersResult) -> Self {
            value.clone()
        }
    }

    ///CreateOrganizationIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationName",
    ///    "rootAuthenticator",
    ///    "rootEmail"
    ///  ],
    ///  "properties": {
    ///    "organizationName": {
    ///      "description": "Human-readable name for an Organization.",
    ///      "type": "string"
    ///    },
    ///    "rootAuthenticator": {
    ///      "$ref": "#/components/schemas/AuthenticatorParams"
    ///    },
    ///    "rootEmail": {
    ///      "description": "The root user's email address.",
    ///      "type": "string"
    ///    },
    ///    "rootUserId": {
    ///      "description": "Unique identifier for the root user object.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateOrganizationIntent {
        ///Human-readable name for an Organization.
        #[serde(rename = "organizationName")]
        pub organization_name: String,
        #[serde(rename = "rootAuthenticator")]
        pub root_authenticator: AuthenticatorParams,
        ///The root user's email address.
        #[serde(rename = "rootEmail")]
        pub root_email: String,
        ///Unique identifier for the root user object.
        #[serde(
            rename = "rootUserId",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub root_user_id: Option<String>,
    }

    impl From<&CreateOrganizationIntent> for CreateOrganizationIntent {
        fn from(value: &CreateOrganizationIntent) -> Self {
            value.clone()
        }
    }

    ///CreateOrganizationIntentV2
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationName",
    ///    "rootAuthenticator",
    ///    "rootEmail"
    ///  ],
    ///  "properties": {
    ///    "organizationName": {
    ///      "description": "Human-readable name for an Organization.",
    ///      "type": "string"
    ///    },
    ///    "rootAuthenticator": {
    ///      "$ref": "#/components/schemas/AuthenticatorParamsV2"
    ///    },
    ///    "rootEmail": {
    ///      "description": "The root user's email address.",
    ///      "type": "string"
    ///    },
    ///    "rootUserId": {
    ///      "description": "Unique identifier for the root user object.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateOrganizationIntentV2 {
        ///Human-readable name for an Organization.
        #[serde(rename = "organizationName")]
        pub organization_name: String,
        #[serde(rename = "rootAuthenticator")]
        pub root_authenticator: AuthenticatorParamsV2,
        ///The root user's email address.
        #[serde(rename = "rootEmail")]
        pub root_email: String,
        ///Unique identifier for the root user object.
        #[serde(
            rename = "rootUserId",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub root_user_id: Option<String>,
    }

    impl From<&CreateOrganizationIntentV2> for CreateOrganizationIntentV2 {
        fn from(value: &CreateOrganizationIntentV2) -> Self {
            value.clone()
        }
    }

    ///CreateOrganizationResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateOrganizationResult {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
    }

    impl From<&CreateOrganizationResult> for CreateOrganizationResult {
        fn from(value: &CreateOrganizationResult) -> Self {
            value.clone()
        }
    }

    ///CreatePoliciesIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "policies"
    ///  ],
    ///  "properties": {
    ///    "policies": {
    ///      "description": "An array of policy intents to be created.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/CreatePolicyIntentV3"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreatePoliciesIntent {
        ///An array of policy intents to be created.
        pub policies: Vec<CreatePolicyIntentV3>,
    }

    impl From<&CreatePoliciesIntent> for CreatePoliciesIntent {
        fn from(value: &CreatePoliciesIntent) -> Self {
            value.clone()
        }
    }

    ///CreatePoliciesRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/CreatePoliciesIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_CREATE_POLICIES"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreatePoliciesRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: CreatePoliciesIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: CreatePoliciesRequestType,
    }

    impl From<&CreatePoliciesRequest> for CreatePoliciesRequest {
        fn from(value: &CreatePoliciesRequest) -> Self {
            value.clone()
        }
    }

    ///CreatePoliciesRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_CREATE_POLICIES"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum CreatePoliciesRequestType {
        #[serde(rename = "ACTIVITY_TYPE_CREATE_POLICIES")]
        ActivityTypeCreatePolicies,
    }

    impl From<&CreatePoliciesRequestType> for CreatePoliciesRequestType {
        fn from(value: &CreatePoliciesRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for CreatePoliciesRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeCreatePolicies => write!(f, "ACTIVITY_TYPE_CREATE_POLICIES"),
            }
        }
    }

    impl std::str::FromStr for CreatePoliciesRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_CREATE_POLICIES" => Ok(Self::ActivityTypeCreatePolicies),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for CreatePoliciesRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for CreatePoliciesRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for CreatePoliciesRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///CreatePoliciesResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "policyIds"
    ///  ],
    ///  "properties": {
    ///    "policyIds": {
    ///      "description": "A list of unique identifiers for the created
    /// policies.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreatePoliciesResult {
        ///A list of unique identifiers for the created policies.
        #[serde(rename = "policyIds")]
        pub policy_ids: Vec<String>,
    }

    impl From<&CreatePoliciesResult> for CreatePoliciesResult {
        fn from(value: &CreatePoliciesResult) -> Self {
            value.clone()
        }
    }

    ///CreatePolicyIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "effect",
    ///    "policyName",
    ///    "selectors"
    ///  ],
    ///  "properties": {
    ///    "effect": {
    ///      "$ref": "#/components/schemas/Effect"
    ///    },
    ///    "notes": {
    ///      "type": "string"
    ///    },
    ///    "policyName": {
    ///      "description": "Human-readable name for a Policy.",
    ///      "type": "string"
    ///    },
    ///    "selectors": {
    ///      "description": "A list of simple functions each including a
    /// subject, target and boolean. See Policy Engine Language section for
    /// additional details.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/Selector"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreatePolicyIntent {
        pub effect: Effect,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub notes: Option<String>,
        ///Human-readable name for a Policy.
        #[serde(rename = "policyName")]
        pub policy_name: String,
        ///A list of simple functions each including a subject, target and
        /// boolean. See Policy Engine Language section for additional details.
        pub selectors: Vec<Selector>,
    }

    impl From<&CreatePolicyIntent> for CreatePolicyIntent {
        fn from(value: &CreatePolicyIntent) -> Self {
            value.clone()
        }
    }

    ///CreatePolicyIntentV2
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "effect",
    ///    "policyName",
    ///    "selectors"
    ///  ],
    ///  "properties": {
    ///    "effect": {
    ///      "$ref": "#/components/schemas/Effect"
    ///    },
    ///    "notes": {
    ///      "type": "string"
    ///    },
    ///    "policyName": {
    ///      "description": "Human-readable name for a Policy.",
    ///      "type": "string"
    ///    },
    ///    "selectors": {
    ///      "description": "A list of simple functions each including a
    /// subject, target and boolean. See Policy Engine Language section for
    /// additional details.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/SelectorV2"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreatePolicyIntentV2 {
        pub effect: Effect,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub notes: Option<String>,
        ///Human-readable name for a Policy.
        #[serde(rename = "policyName")]
        pub policy_name: String,
        ///A list of simple functions each including a subject, target and
        /// boolean. See Policy Engine Language section for additional details.
        pub selectors: Vec<SelectorV2>,
    }

    impl From<&CreatePolicyIntentV2> for CreatePolicyIntentV2 {
        fn from(value: &CreatePolicyIntentV2) -> Self {
            value.clone()
        }
    }

    ///CreatePolicyIntentV3
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "effect",
    ///    "policyName"
    ///  ],
    ///  "properties": {
    ///    "condition": {
    ///      "description": "The condition expression that triggers the Effect",
    ///      "type": "string"
    ///    },
    ///    "consensus": {
    ///      "description": "The consensus expression that triggers the Effect",
    ///      "type": "string"
    ///    },
    ///    "effect": {
    ///      "$ref": "#/components/schemas/Effect"
    ///    },
    ///    "notes": {
    ///      "type": "string"
    ///    },
    ///    "policyName": {
    ///      "description": "Human-readable name for a Policy.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreatePolicyIntentV3 {
        ///The condition expression that triggers the Effect
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub condition: Option<String>,
        ///The consensus expression that triggers the Effect
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub consensus: Option<String>,
        pub effect: Effect,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub notes: Option<String>,
        ///Human-readable name for a Policy.
        #[serde(rename = "policyName")]
        pub policy_name: String,
    }

    impl From<&CreatePolicyIntentV3> for CreatePolicyIntentV3 {
        fn from(value: &CreatePolicyIntentV3) -> Self {
            value.clone()
        }
    }

    ///CreatePolicyRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/CreatePolicyIntentV3"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_CREATE_POLICY_V3"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreatePolicyRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: CreatePolicyIntentV3,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: CreatePolicyRequestType,
    }

    impl From<&CreatePolicyRequest> for CreatePolicyRequest {
        fn from(value: &CreatePolicyRequest) -> Self {
            value.clone()
        }
    }

    ///CreatePolicyRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_CREATE_POLICY_V3"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum CreatePolicyRequestType {
        #[serde(rename = "ACTIVITY_TYPE_CREATE_POLICY_V3")]
        ActivityTypeCreatePolicyV3,
    }

    impl From<&CreatePolicyRequestType> for CreatePolicyRequestType {
        fn from(value: &CreatePolicyRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for CreatePolicyRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeCreatePolicyV3 => write!(f, "ACTIVITY_TYPE_CREATE_POLICY_V3"),
            }
        }
    }

    impl std::str::FromStr for CreatePolicyRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_CREATE_POLICY_V3" => Ok(Self::ActivityTypeCreatePolicyV3),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for CreatePolicyRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for CreatePolicyRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for CreatePolicyRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///CreatePolicyResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "policyId"
    ///  ],
    ///  "properties": {
    ///    "policyId": {
    ///      "description": "Unique identifier for a given Policy.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreatePolicyResult {
        ///Unique identifier for a given Policy.
        #[serde(rename = "policyId")]
        pub policy_id: String,
    }

    impl From<&CreatePolicyResult> for CreatePolicyResult {
        fn from(value: &CreatePolicyResult) -> Self {
            value.clone()
        }
    }

    ///CreatePrivateKeyTagIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "privateKeyIds",
    ///    "privateKeyTagName"
    ///  ],
    ///  "properties": {
    ///    "privateKeyIds": {
    ///      "description": "A list of Private Key IDs.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "privateKeyTagName": {
    ///      "description": "Human-readable name for a Private Key Tag.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreatePrivateKeyTagIntent {
        ///A list of Private Key IDs.
        #[serde(rename = "privateKeyIds")]
        pub private_key_ids: Vec<String>,
        ///Human-readable name for a Private Key Tag.
        #[serde(rename = "privateKeyTagName")]
        pub private_key_tag_name: String,
    }

    impl From<&CreatePrivateKeyTagIntent> for CreatePrivateKeyTagIntent {
        fn from(value: &CreatePrivateKeyTagIntent) -> Self {
            value.clone()
        }
    }

    ///CreatePrivateKeyTagRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/CreatePrivateKeyTagIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_CREATE_PRIVATE_KEY_TAG"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreatePrivateKeyTagRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: CreatePrivateKeyTagIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: CreatePrivateKeyTagRequestType,
    }

    impl From<&CreatePrivateKeyTagRequest> for CreatePrivateKeyTagRequest {
        fn from(value: &CreatePrivateKeyTagRequest) -> Self {
            value.clone()
        }
    }

    ///CreatePrivateKeyTagRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_CREATE_PRIVATE_KEY_TAG"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum CreatePrivateKeyTagRequestType {
        #[serde(rename = "ACTIVITY_TYPE_CREATE_PRIVATE_KEY_TAG")]
        ActivityTypeCreatePrivateKeyTag,
    }

    impl From<&CreatePrivateKeyTagRequestType> for CreatePrivateKeyTagRequestType {
        fn from(value: &CreatePrivateKeyTagRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for CreatePrivateKeyTagRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeCreatePrivateKeyTag => {
                    write!(f, "ACTIVITY_TYPE_CREATE_PRIVATE_KEY_TAG")
                }
            }
        }
    }

    impl std::str::FromStr for CreatePrivateKeyTagRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_CREATE_PRIVATE_KEY_TAG" => Ok(Self::ActivityTypeCreatePrivateKeyTag),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for CreatePrivateKeyTagRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for CreatePrivateKeyTagRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for CreatePrivateKeyTagRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///CreatePrivateKeyTagResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "privateKeyIds",
    ///    "privateKeyTagId"
    ///  ],
    ///  "properties": {
    ///    "privateKeyIds": {
    ///      "description": "A list of Private Key IDs.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "privateKeyTagId": {
    ///      "description": "Unique identifier for a given Private Key Tag.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreatePrivateKeyTagResult {
        ///A list of Private Key IDs.
        #[serde(rename = "privateKeyIds")]
        pub private_key_ids: Vec<String>,
        ///Unique identifier for a given Private Key Tag.
        #[serde(rename = "privateKeyTagId")]
        pub private_key_tag_id: String,
    }

    impl From<&CreatePrivateKeyTagResult> for CreatePrivateKeyTagResult {
        fn from(value: &CreatePrivateKeyTagResult) -> Self {
            value.clone()
        }
    }

    ///CreatePrivateKeysIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "privateKeys"
    ///  ],
    ///  "properties": {
    ///    "privateKeys": {
    ///      "description": "A list of Private Keys.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/PrivateKeyParams"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreatePrivateKeysIntent {
        ///A list of Private Keys.
        #[serde(rename = "privateKeys")]
        pub private_keys: Vec<PrivateKeyParams>,
    }

    impl From<&CreatePrivateKeysIntent> for CreatePrivateKeysIntent {
        fn from(value: &CreatePrivateKeysIntent) -> Self {
            value.clone()
        }
    }

    ///CreatePrivateKeysIntentV2
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "privateKeys"
    ///  ],
    ///  "properties": {
    ///    "privateKeys": {
    ///      "description": "A list of Private Keys.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/PrivateKeyParams"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreatePrivateKeysIntentV2 {
        ///A list of Private Keys.
        #[serde(rename = "privateKeys")]
        pub private_keys: Vec<PrivateKeyParams>,
    }

    impl From<&CreatePrivateKeysIntentV2> for CreatePrivateKeysIntentV2 {
        fn from(value: &CreatePrivateKeysIntentV2) -> Self {
            value.clone()
        }
    }

    ///CreatePrivateKeysRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/CreatePrivateKeysIntentV2"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_CREATE_PRIVATE_KEYS_V2"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreatePrivateKeysRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: CreatePrivateKeysIntentV2,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: CreatePrivateKeysRequestType,
    }

    impl From<&CreatePrivateKeysRequest> for CreatePrivateKeysRequest {
        fn from(value: &CreatePrivateKeysRequest) -> Self {
            value.clone()
        }
    }

    ///CreatePrivateKeysRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_CREATE_PRIVATE_KEYS_V2"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum CreatePrivateKeysRequestType {
        #[serde(rename = "ACTIVITY_TYPE_CREATE_PRIVATE_KEYS_V2")]
        ActivityTypeCreatePrivateKeysV2,
    }

    impl From<&CreatePrivateKeysRequestType> for CreatePrivateKeysRequestType {
        fn from(value: &CreatePrivateKeysRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for CreatePrivateKeysRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeCreatePrivateKeysV2 => {
                    write!(f, "ACTIVITY_TYPE_CREATE_PRIVATE_KEYS_V2")
                }
            }
        }
    }

    impl std::str::FromStr for CreatePrivateKeysRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_CREATE_PRIVATE_KEYS_V2" => Ok(Self::ActivityTypeCreatePrivateKeysV2),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for CreatePrivateKeysRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for CreatePrivateKeysRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for CreatePrivateKeysRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///CreatePrivateKeysResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "privateKeyIds"
    ///  ],
    ///  "properties": {
    ///    "privateKeyIds": {
    ///      "description": "A list of Private Key IDs.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreatePrivateKeysResult {
        ///A list of Private Key IDs.
        #[serde(rename = "privateKeyIds")]
        pub private_key_ids: Vec<String>,
    }

    impl From<&CreatePrivateKeysResult> for CreatePrivateKeysResult {
        fn from(value: &CreatePrivateKeysResult) -> Self {
            value.clone()
        }
    }

    ///CreatePrivateKeysResultV2
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "privateKeys"
    ///  ],
    ///  "properties": {
    ///    "privateKeys": {
    ///      "description": "A list of Private Key IDs and addresses.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/PrivateKeyResult"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreatePrivateKeysResultV2 {
        ///A list of Private Key IDs and addresses.
        #[serde(rename = "privateKeys")]
        pub private_keys: Vec<PrivateKeyResult>,
    }

    impl From<&CreatePrivateKeysResultV2> for CreatePrivateKeysResultV2 {
        fn from(value: &CreatePrivateKeysResultV2) -> Self {
            value.clone()
        }
    }

    ///CreateReadOnlySessionIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object"
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateReadOnlySessionIntent(pub ::serde_json::Map<String, ::serde_json::Value>);
    impl ::std::ops::Deref for CreateReadOnlySessionIntent {
        type Target = ::serde_json::Map<String, ::serde_json::Value>;
        fn deref(&self) -> &::serde_json::Map<String, ::serde_json::Value> {
            &self.0
        }
    }

    impl From<CreateReadOnlySessionIntent> for ::serde_json::Map<String, ::serde_json::Value> {
        fn from(value: CreateReadOnlySessionIntent) -> Self {
            value.0
        }
    }

    impl From<&CreateReadOnlySessionIntent> for CreateReadOnlySessionIntent {
        fn from(value: &CreateReadOnlySessionIntent) -> Self {
            value.clone()
        }
    }

    impl From<::serde_json::Map<String, ::serde_json::Value>> for CreateReadOnlySessionIntent {
        fn from(value: ::serde_json::Map<String, ::serde_json::Value>) -> Self {
            Self(value)
        }
    }

    ///CreateReadOnlySessionRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/CreateReadOnlySessionIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_CREATE_READ_ONLY_SESSION"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateReadOnlySessionRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: CreateReadOnlySessionIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: CreateReadOnlySessionRequestType,
    }

    impl From<&CreateReadOnlySessionRequest> for CreateReadOnlySessionRequest {
        fn from(value: &CreateReadOnlySessionRequest) -> Self {
            value.clone()
        }
    }

    ///CreateReadOnlySessionRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_CREATE_READ_ONLY_SESSION"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum CreateReadOnlySessionRequestType {
        #[serde(rename = "ACTIVITY_TYPE_CREATE_READ_ONLY_SESSION")]
        ActivityTypeCreateReadOnlySession,
    }

    impl From<&CreateReadOnlySessionRequestType> for CreateReadOnlySessionRequestType {
        fn from(value: &CreateReadOnlySessionRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for CreateReadOnlySessionRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeCreateReadOnlySession => {
                    write!(f, "ACTIVITY_TYPE_CREATE_READ_ONLY_SESSION")
                }
            }
        }
    }

    impl std::str::FromStr for CreateReadOnlySessionRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_CREATE_READ_ONLY_SESSION" => {
                    Ok(Self::ActivityTypeCreateReadOnlySession)
                }
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for CreateReadOnlySessionRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for CreateReadOnlySessionRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for CreateReadOnlySessionRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///CreateReadOnlySessionResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "organizationName",
    ///    "session",
    ///    "sessionExpiry",
    ///    "userId",
    ///    "username"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization. If the
    /// request is being made by a user and their Sub-Organization ID is
    /// unknown, this can be the Parent Organization ID. However, using the
    /// Sub-Organization ID is preferred due to performance reasons.",
    ///      "type": "string"
    ///    },
    ///    "organizationName": {
    ///      "description": "Human-readable name for an Organization.",
    ///      "type": "string"
    ///    },
    ///    "session": {
    ///      "description": "String representing a read only session",
    ///      "type": "string"
    ///    },
    ///    "sessionExpiry": {
    ///      "description": "UTC timestamp in seconds representing the expiry
    /// time for the read only session.",
    ///      "type": "string",
    ///      "format": "uint64"
    ///    },
    ///    "userId": {
    ///      "description": "Unique identifier for a given User.",
    ///      "type": "string"
    ///    },
    ///    "username": {
    ///      "description": "Human-readable name for a User.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateReadOnlySessionResult {
        ///Unique identifier for a given Organization. If the request is being
        /// made by a user and their Sub-Organization ID is unknown, this can be
        /// the Parent Organization ID. However, using the Sub-Organization ID
        /// is preferred due to performance reasons.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        ///Human-readable name for an Organization.
        #[serde(rename = "organizationName")]
        pub organization_name: String,
        ///String representing a read only session
        pub session: String,
        ///UTC timestamp in seconds representing the expiry time for the read
        /// only session.
        #[serde(rename = "sessionExpiry")]
        pub session_expiry: String,
        ///Unique identifier for a given User.
        #[serde(rename = "userId")]
        pub user_id: String,
        ///Human-readable name for a User.
        pub username: String,
    }

    impl From<&CreateReadOnlySessionResult> for CreateReadOnlySessionResult {
        fn from(value: &CreateReadOnlySessionResult) -> Self {
            value.clone()
        }
    }

    ///CreateReadWriteSessionIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "email",
    ///    "targetPublicKey"
    ///  ],
    ///  "properties": {
    ///    "apiKeyName": {
    ///      "description": "Optional human-readable name for an API Key. If
    /// none provided, default to Read Write Session - <Timestamp>",
    ///      "type": "string"
    ///    },
    ///    "email": {
    ///      "description": "Email of the user to create a read write session
    /// for",
    ///      "type": "string"
    ///    },
    ///    "expirationSeconds": {
    ///      "description": "Expiration window (in seconds) indicating how long
    /// the API key is valid. If not provided, a default of 15 minutes will be
    /// used.",
    ///      "type": "string"
    ///    },
    ///    "targetPublicKey": {
    ///      "description": "Client-side public key generated by the user, to which the read write session bundle (credentials) will be encrypted.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateReadWriteSessionIntent {
        ///Optional human-readable name for an API Key. If none provided,
        /// default to Read Write Session - <Timestamp>
        #[serde(
            rename = "apiKeyName",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub api_key_name: Option<String>,
        ///Email of the user to create a read write session for
        pub email: String,
        ///Expiration window (in seconds) indicating how long the API key is
        /// valid. If not provided, a default of 15 minutes will be used.
        #[serde(
            rename = "expirationSeconds",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub expiration_seconds: Option<String>,
        ///Client-side public key generated by the user, to which the read
        /// write session bundle (credentials) will be encrypted.
        #[serde(rename = "targetPublicKey")]
        pub target_public_key: String,
    }

    impl From<&CreateReadWriteSessionIntent> for CreateReadWriteSessionIntent {
        fn from(value: &CreateReadWriteSessionIntent) -> Self {
            value.clone()
        }
    }

    ///CreateReadWriteSessionIntentV2
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "targetPublicKey"
    ///  ],
    ///  "properties": {
    ///    "apiKeyName": {
    ///      "description": "Optional human-readable name for an API Key. If
    /// none provided, default to Read Write Session - <Timestamp>",
    ///      "type": "string"
    ///    },
    ///    "expirationSeconds": {
    ///      "description": "Expiration window (in seconds) indicating how long
    /// the API key is valid. If not provided, a default of 15 minutes will be
    /// used.",
    ///      "type": "string"
    ///    },
    ///    "targetPublicKey": {
    ///      "description": "Client-side public key generated by the user, to which the read write session bundle (credentials) will be encrypted.",
    ///      "type": "string"
    ///    },
    ///    "userId": {
    ///      "description": "Unique identifier for a given User.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateReadWriteSessionIntentV2 {
        ///Optional human-readable name for an API Key. If none provided,
        /// default to Read Write Session - <Timestamp>
        #[serde(
            rename = "apiKeyName",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub api_key_name: Option<String>,
        ///Expiration window (in seconds) indicating how long the API key is
        /// valid. If not provided, a default of 15 minutes will be used.
        #[serde(
            rename = "expirationSeconds",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub expiration_seconds: Option<String>,
        ///Client-side public key generated by the user, to which the read
        /// write session bundle (credentials) will be encrypted.
        #[serde(rename = "targetPublicKey")]
        pub target_public_key: String,
        ///Unique identifier for a given User.
        #[serde(rename = "userId", default, skip_serializing_if = "Option::is_none")]
        pub user_id: Option<String>,
    }

    impl From<&CreateReadWriteSessionIntentV2> for CreateReadWriteSessionIntentV2 {
        fn from(value: &CreateReadWriteSessionIntentV2) -> Self {
            value.clone()
        }
    }

    ///CreateReadWriteSessionRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/CreateReadWriteSessionIntentV2"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_CREATE_READ_WRITE_SESSION_V2"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateReadWriteSessionRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: CreateReadWriteSessionIntentV2,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: CreateReadWriteSessionRequestType,
    }

    impl From<&CreateReadWriteSessionRequest> for CreateReadWriteSessionRequest {
        fn from(value: &CreateReadWriteSessionRequest) -> Self {
            value.clone()
        }
    }

    ///CreateReadWriteSessionRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_CREATE_READ_WRITE_SESSION_V2"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum CreateReadWriteSessionRequestType {
        #[serde(rename = "ACTIVITY_TYPE_CREATE_READ_WRITE_SESSION_V2")]
        ActivityTypeCreateReadWriteSessionV2,
    }

    impl From<&CreateReadWriteSessionRequestType> for CreateReadWriteSessionRequestType {
        fn from(value: &CreateReadWriteSessionRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for CreateReadWriteSessionRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeCreateReadWriteSessionV2 => {
                    write!(f, "ACTIVITY_TYPE_CREATE_READ_WRITE_SESSION_V2")
                }
            }
        }
    }

    impl std::str::FromStr for CreateReadWriteSessionRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_CREATE_READ_WRITE_SESSION_V2" => {
                    Ok(Self::ActivityTypeCreateReadWriteSessionV2)
                }
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for CreateReadWriteSessionRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for CreateReadWriteSessionRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for CreateReadWriteSessionRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///CreateReadWriteSessionResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "apiKeyId",
    ///    "credentialBundle",
    ///    "organizationId",
    ///    "organizationName",
    ///    "userId",
    ///    "username"
    ///  ],
    ///  "properties": {
    ///    "apiKeyId": {
    ///      "description": "Unique identifier for the created API key.",
    ///      "type": "string"
    ///    },
    ///    "credentialBundle": {
    ///      "description": "HPKE encrypted credential bundle",
    ///      "type": "string"
    ///    },
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization. If the
    /// request is being made by a user and their Sub-Organization ID is
    /// unknown, this can be the Parent Organization ID. However, using the
    /// Sub-Organization ID is preferred due to performance reasons.",
    ///      "type": "string"
    ///    },
    ///    "organizationName": {
    ///      "description": "Human-readable name for an Organization.",
    ///      "type": "string"
    ///    },
    ///    "userId": {
    ///      "description": "Unique identifier for a given User.",
    ///      "type": "string"
    ///    },
    ///    "username": {
    ///      "description": "Human-readable name for a User.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateReadWriteSessionResult {
        ///Unique identifier for the created API key.
        #[serde(rename = "apiKeyId")]
        pub api_key_id: String,
        ///HPKE encrypted credential bundle
        #[serde(rename = "credentialBundle")]
        pub credential_bundle: String,
        ///Unique identifier for a given Organization. If the request is being
        /// made by a user and their Sub-Organization ID is unknown, this can be
        /// the Parent Organization ID. However, using the Sub-Organization ID
        /// is preferred due to performance reasons.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        ///Human-readable name for an Organization.
        #[serde(rename = "organizationName")]
        pub organization_name: String,
        ///Unique identifier for a given User.
        #[serde(rename = "userId")]
        pub user_id: String,
        ///Human-readable name for a User.
        pub username: String,
    }

    impl From<&CreateReadWriteSessionResult> for CreateReadWriteSessionResult {
        fn from(value: &CreateReadWriteSessionResult) -> Self {
            value.clone()
        }
    }

    ///CreateReadWriteSessionResultV2
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "apiKeyId",
    ///    "credentialBundle",
    ///    "organizationId",
    ///    "organizationName",
    ///    "userId",
    ///    "username"
    ///  ],
    ///  "properties": {
    ///    "apiKeyId": {
    ///      "description": "Unique identifier for the created API key.",
    ///      "type": "string"
    ///    },
    ///    "credentialBundle": {
    ///      "description": "HPKE encrypted credential bundle",
    ///      "type": "string"
    ///    },
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization. If the
    /// request is being made by a user and their Sub-Organization ID is
    /// unknown, this can be the Parent Organization ID. However, using the
    /// Sub-Organization ID is preferred due to performance reasons.",
    ///      "type": "string"
    ///    },
    ///    "organizationName": {
    ///      "description": "Human-readable name for an Organization.",
    ///      "type": "string"
    ///    },
    ///    "userId": {
    ///      "description": "Unique identifier for a given User.",
    ///      "type": "string"
    ///    },
    ///    "username": {
    ///      "description": "Human-readable name for a User.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateReadWriteSessionResultV2 {
        ///Unique identifier for the created API key.
        #[serde(rename = "apiKeyId")]
        pub api_key_id: String,
        ///HPKE encrypted credential bundle
        #[serde(rename = "credentialBundle")]
        pub credential_bundle: String,
        ///Unique identifier for a given Organization. If the request is being
        /// made by a user and their Sub-Organization ID is unknown, this can be
        /// the Parent Organization ID. However, using the Sub-Organization ID
        /// is preferred due to performance reasons.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        ///Human-readable name for an Organization.
        #[serde(rename = "organizationName")]
        pub organization_name: String,
        ///Unique identifier for a given User.
        #[serde(rename = "userId")]
        pub user_id: String,
        ///Human-readable name for a User.
        pub username: String,
    }

    impl From<&CreateReadWriteSessionResultV2> for CreateReadWriteSessionResultV2 {
        fn from(value: &CreateReadWriteSessionResultV2) -> Self {
            value.clone()
        }
    }

    ///CreateSubOrganizationIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "name",
    ///    "rootAuthenticator"
    ///  ],
    ///  "properties": {
    ///    "name": {
    ///      "description": "Name for this sub-organization",
    ///      "type": "string"
    ///    },
    ///    "rootAuthenticator": {
    ///      "$ref": "#/components/schemas/AuthenticatorParamsV2"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateSubOrganizationIntent {
        ///Name for this sub-organization
        pub name: String,
        #[serde(rename = "rootAuthenticator")]
        pub root_authenticator: AuthenticatorParamsV2,
    }

    impl From<&CreateSubOrganizationIntent> for CreateSubOrganizationIntent {
        fn from(value: &CreateSubOrganizationIntent) -> Self {
            value.clone()
        }
    }

    ///CreateSubOrganizationIntentV2
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "rootQuorumThreshold",
    ///    "rootUsers",
    ///    "subOrganizationName"
    ///  ],
    ///  "properties": {
    ///    "rootQuorumThreshold": {
    ///      "description": "The threshold of unique approvals to reach root
    /// quorum. This value must be less than or equal to the number of root
    /// users",
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "rootUsers": {
    ///      "description": "Root users to create within this sub-organization",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/RootUserParams"
    ///      }
    ///    },
    ///    "subOrganizationName": {
    ///      "description": "Name for this sub-organization",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateSubOrganizationIntentV2 {
        ///The threshold of unique approvals to reach root quorum. This value
        /// must be less than or equal to the number of root users
        #[serde(rename = "rootQuorumThreshold")]
        pub root_quorum_threshold: i32,
        ///Root users to create within this sub-organization
        #[serde(rename = "rootUsers")]
        pub root_users: Vec<RootUserParams>,
        ///Name for this sub-organization
        #[serde(rename = "subOrganizationName")]
        pub sub_organization_name: String,
    }

    impl From<&CreateSubOrganizationIntentV2> for CreateSubOrganizationIntentV2 {
        fn from(value: &CreateSubOrganizationIntentV2) -> Self {
            value.clone()
        }
    }

    ///CreateSubOrganizationIntentV3
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "privateKeys",
    ///    "rootQuorumThreshold",
    ///    "rootUsers",
    ///    "subOrganizationName"
    ///  ],
    ///  "properties": {
    ///    "privateKeys": {
    ///      "description": "A list of Private Keys.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/PrivateKeyParams"
    ///      }
    ///    },
    ///    "rootQuorumThreshold": {
    ///      "description": "The threshold of unique approvals to reach root
    /// quorum. This value must be less than or equal to the number of root
    /// users",
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "rootUsers": {
    ///      "description": "Root users to create within this sub-organization",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/RootUserParams"
    ///      }
    ///    },
    ///    "subOrganizationName": {
    ///      "description": "Name for this sub-organization",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateSubOrganizationIntentV3 {
        ///A list of Private Keys.
        #[serde(rename = "privateKeys")]
        pub private_keys: Vec<PrivateKeyParams>,
        ///The threshold of unique approvals to reach root quorum. This value
        /// must be less than or equal to the number of root users
        #[serde(rename = "rootQuorumThreshold")]
        pub root_quorum_threshold: i32,
        ///Root users to create within this sub-organization
        #[serde(rename = "rootUsers")]
        pub root_users: Vec<RootUserParams>,
        ///Name for this sub-organization
        #[serde(rename = "subOrganizationName")]
        pub sub_organization_name: String,
    }

    impl From<&CreateSubOrganizationIntentV3> for CreateSubOrganizationIntentV3 {
        fn from(value: &CreateSubOrganizationIntentV3) -> Self {
            value.clone()
        }
    }

    ///CreateSubOrganizationIntentV4
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "rootQuorumThreshold",
    ///    "rootUsers",
    ///    "subOrganizationName"
    ///  ],
    ///  "properties": {
    ///    "disableEmailAuth": {
    ///      "description": "Disable email auth for the sub-organization",
    ///      "type": "boolean"
    ///    },
    ///    "disableEmailRecovery": {
    ///      "description": "Disable email recovery for the sub-organization",
    ///      "type": "boolean"
    ///    },
    ///    "rootQuorumThreshold": {
    ///      "description": "The threshold of unique approvals to reach root
    /// quorum. This value must be less than or equal to the number of root
    /// users",
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "rootUsers": {
    ///      "description": "Root users to create within this sub-organization",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/RootUserParams"
    ///      }
    ///    },
    ///    "subOrganizationName": {
    ///      "description": "Name for this sub-organization",
    ///      "type": "string"
    ///    },
    ///    "wallet": {
    ///      "$ref": "#/components/schemas/WalletParams"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateSubOrganizationIntentV4 {
        ///Disable email auth for the sub-organization
        #[serde(
            rename = "disableEmailAuth",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub disable_email_auth: Option<bool>,
        ///Disable email recovery for the sub-organization
        #[serde(
            rename = "disableEmailRecovery",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub disable_email_recovery: Option<bool>,
        ///The threshold of unique approvals to reach root quorum. This value
        /// must be less than or equal to the number of root users
        #[serde(rename = "rootQuorumThreshold")]
        pub root_quorum_threshold: i32,
        ///Root users to create within this sub-organization
        #[serde(rename = "rootUsers")]
        pub root_users: Vec<RootUserParams>,
        ///Name for this sub-organization
        #[serde(rename = "subOrganizationName")]
        pub sub_organization_name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub wallet: Option<WalletParams>,
    }

    impl From<&CreateSubOrganizationIntentV4> for CreateSubOrganizationIntentV4 {
        fn from(value: &CreateSubOrganizationIntentV4) -> Self {
            value.clone()
        }
    }

    ///CreateSubOrganizationIntentV5
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "rootQuorumThreshold",
    ///    "rootUsers",
    ///    "subOrganizationName"
    ///  ],
    ///  "properties": {
    ///    "disableEmailAuth": {
    ///      "description": "Disable email auth for the sub-organization",
    ///      "type": "boolean"
    ///    },
    ///    "disableEmailRecovery": {
    ///      "description": "Disable email recovery for the sub-organization",
    ///      "type": "boolean"
    ///    },
    ///    "rootQuorumThreshold": {
    ///      "description": "The threshold of unique approvals to reach root
    /// quorum. This value must be less than or equal to the number of root
    /// users",
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "rootUsers": {
    ///      "description": "Root users to create within this sub-organization",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/RootUserParamsV2"
    ///      }
    ///    },
    ///    "subOrganizationName": {
    ///      "description": "Name for this sub-organization",
    ///      "type": "string"
    ///    },
    ///    "wallet": {
    ///      "$ref": "#/components/schemas/WalletParams"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateSubOrganizationIntentV5 {
        ///Disable email auth for the sub-organization
        #[serde(
            rename = "disableEmailAuth",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub disable_email_auth: Option<bool>,
        ///Disable email recovery for the sub-organization
        #[serde(
            rename = "disableEmailRecovery",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub disable_email_recovery: Option<bool>,
        ///The threshold of unique approvals to reach root quorum. This value
        /// must be less than or equal to the number of root users
        #[serde(rename = "rootQuorumThreshold")]
        pub root_quorum_threshold: i32,
        ///Root users to create within this sub-organization
        #[serde(rename = "rootUsers")]
        pub root_users: Vec<RootUserParamsV2>,
        ///Name for this sub-organization
        #[serde(rename = "subOrganizationName")]
        pub sub_organization_name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub wallet: Option<WalletParams>,
    }

    impl From<&CreateSubOrganizationIntentV5> for CreateSubOrganizationIntentV5 {
        fn from(value: &CreateSubOrganizationIntentV5) -> Self {
            value.clone()
        }
    }

    ///CreateSubOrganizationIntentV6
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "rootQuorumThreshold",
    ///    "rootUsers",
    ///    "subOrganizationName"
    ///  ],
    ///  "properties": {
    ///    "disableEmailAuth": {
    ///      "description": "Disable email auth for the sub-organization",
    ///      "type": "boolean"
    ///    },
    ///    "disableEmailRecovery": {
    ///      "description": "Disable email recovery for the sub-organization",
    ///      "type": "boolean"
    ///    },
    ///    "rootQuorumThreshold": {
    ///      "description": "The threshold of unique approvals to reach root
    /// quorum. This value must be less than or equal to the number of root
    /// users",
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "rootUsers": {
    ///      "description": "Root users to create within this sub-organization",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/RootUserParamsV3"
    ///      }
    ///    },
    ///    "subOrganizationName": {
    ///      "description": "Name for this sub-organization",
    ///      "type": "string"
    ///    },
    ///    "wallet": {
    ///      "$ref": "#/components/schemas/WalletParams"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateSubOrganizationIntentV6 {
        ///Disable email auth for the sub-organization
        #[serde(
            rename = "disableEmailAuth",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub disable_email_auth: Option<bool>,
        ///Disable email recovery for the sub-organization
        #[serde(
            rename = "disableEmailRecovery",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub disable_email_recovery: Option<bool>,
        ///The threshold of unique approvals to reach root quorum. This value
        /// must be less than or equal to the number of root users
        #[serde(rename = "rootQuorumThreshold")]
        pub root_quorum_threshold: i32,
        ///Root users to create within this sub-organization
        #[serde(rename = "rootUsers")]
        pub root_users: Vec<RootUserParamsV3>,
        ///Name for this sub-organization
        #[serde(rename = "subOrganizationName")]
        pub sub_organization_name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub wallet: Option<WalletParams>,
    }

    impl From<&CreateSubOrganizationIntentV6> for CreateSubOrganizationIntentV6 {
        fn from(value: &CreateSubOrganizationIntentV6) -> Self {
            value.clone()
        }
    }

    ///CreateSubOrganizationIntentV7
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "rootQuorumThreshold",
    ///    "rootUsers",
    ///    "subOrganizationName"
    ///  ],
    ///  "properties": {
    ///    "disableEmailAuth": {
    ///      "description": "Disable email auth for the sub-organization",
    ///      "type": "boolean"
    ///    },
    ///    "disableEmailRecovery": {
    ///      "description": "Disable email recovery for the sub-organization",
    ///      "type": "boolean"
    ///    },
    ///    "disableOtpEmailAuth": {
    ///      "description": "Disable OTP email auth for the sub-organization",
    ///      "type": "boolean"
    ///    },
    ///    "disableSmsAuth": {
    ///      "description": "Disable OTP SMS auth for the sub-organization",
    ///      "type": "boolean"
    ///    },
    ///    "rootQuorumThreshold": {
    ///      "description": "The threshold of unique approvals to reach root
    /// quorum. This value must be less than or equal to the number of root
    /// users",
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "rootUsers": {
    ///      "description": "Root users to create within this sub-organization",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/RootUserParamsV4"
    ///      }
    ///    },
    ///    "subOrganizationName": {
    ///      "description": "Name for this sub-organization",
    ///      "type": "string"
    ///    },
    ///    "wallet": {
    ///      "$ref": "#/components/schemas/WalletParams"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateSubOrganizationIntentV7 {
        ///Disable email auth for the sub-organization
        #[serde(
            rename = "disableEmailAuth",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub disable_email_auth: Option<bool>,
        ///Disable email recovery for the sub-organization
        #[serde(
            rename = "disableEmailRecovery",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub disable_email_recovery: Option<bool>,
        ///Disable OTP email auth for the sub-organization
        #[serde(
            rename = "disableOtpEmailAuth",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub disable_otp_email_auth: Option<bool>,
        ///Disable OTP SMS auth for the sub-organization
        #[serde(
            rename = "disableSmsAuth",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub disable_sms_auth: Option<bool>,
        ///The threshold of unique approvals to reach root quorum. This value
        /// must be less than or equal to the number of root users
        #[serde(rename = "rootQuorumThreshold")]
        pub root_quorum_threshold: i32,
        ///Root users to create within this sub-organization
        #[serde(rename = "rootUsers")]
        pub root_users: Vec<RootUserParamsV4>,
        ///Name for this sub-organization
        #[serde(rename = "subOrganizationName")]
        pub sub_organization_name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub wallet: Option<WalletParams>,
    }

    impl From<&CreateSubOrganizationIntentV7> for CreateSubOrganizationIntentV7 {
        fn from(value: &CreateSubOrganizationIntentV7) -> Self {
            value.clone()
        }
    }

    ///CreateSubOrganizationRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/CreateSubOrganizationIntentV7"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V7"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateSubOrganizationRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: CreateSubOrganizationIntentV7,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: CreateSubOrganizationRequestType,
    }

    impl From<&CreateSubOrganizationRequest> for CreateSubOrganizationRequest {
        fn from(value: &CreateSubOrganizationRequest) -> Self {
            value.clone()
        }
    }

    ///CreateSubOrganizationRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V7"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum CreateSubOrganizationRequestType {
        #[serde(rename = "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V7")]
        ActivityTypeCreateSubOrganizationV7,
    }

    impl From<&CreateSubOrganizationRequestType> for CreateSubOrganizationRequestType {
        fn from(value: &CreateSubOrganizationRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for CreateSubOrganizationRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeCreateSubOrganizationV7 => {
                    write!(f, "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V7")
                }
            }
        }
    }

    impl std::str::FromStr for CreateSubOrganizationRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_CREATE_SUB_ORGANIZATION_V7" => {
                    Ok(Self::ActivityTypeCreateSubOrganizationV7)
                }
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for CreateSubOrganizationRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for CreateSubOrganizationRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for CreateSubOrganizationRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///CreateSubOrganizationResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "subOrganizationId"
    ///  ],
    ///  "properties": {
    ///    "rootUserIds": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "subOrganizationId": {
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateSubOrganizationResult {
        #[serde(rename = "rootUserIds", default, skip_serializing_if = "Vec::is_empty")]
        pub root_user_ids: Vec<String>,
        #[serde(rename = "subOrganizationId")]
        pub sub_organization_id: String,
    }

    impl From<&CreateSubOrganizationResult> for CreateSubOrganizationResult {
        fn from(value: &CreateSubOrganizationResult) -> Self {
            value.clone()
        }
    }

    ///CreateSubOrganizationResultV3
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "privateKeys",
    ///    "subOrganizationId"
    ///  ],
    ///  "properties": {
    ///    "privateKeys": {
    ///      "description": "A list of Private Key IDs and addresses.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/PrivateKeyResult"
    ///      }
    ///    },
    ///    "rootUserIds": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "subOrganizationId": {
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateSubOrganizationResultV3 {
        ///A list of Private Key IDs and addresses.
        #[serde(rename = "privateKeys")]
        pub private_keys: Vec<PrivateKeyResult>,
        #[serde(rename = "rootUserIds", default, skip_serializing_if = "Vec::is_empty")]
        pub root_user_ids: Vec<String>,
        #[serde(rename = "subOrganizationId")]
        pub sub_organization_id: String,
    }

    impl From<&CreateSubOrganizationResultV3> for CreateSubOrganizationResultV3 {
        fn from(value: &CreateSubOrganizationResultV3) -> Self {
            value.clone()
        }
    }

    ///CreateSubOrganizationResultV4
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "subOrganizationId"
    ///  ],
    ///  "properties": {
    ///    "rootUserIds": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "subOrganizationId": {
    ///      "type": "string"
    ///    },
    ///    "wallet": {
    ///      "$ref": "#/components/schemas/WalletResult"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateSubOrganizationResultV4 {
        #[serde(rename = "rootUserIds", default, skip_serializing_if = "Vec::is_empty")]
        pub root_user_ids: Vec<String>,
        #[serde(rename = "subOrganizationId")]
        pub sub_organization_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub wallet: Option<WalletResult>,
    }

    impl From<&CreateSubOrganizationResultV4> for CreateSubOrganizationResultV4 {
        fn from(value: &CreateSubOrganizationResultV4) -> Self {
            value.clone()
        }
    }

    ///CreateSubOrganizationResultV5
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "subOrganizationId"
    ///  ],
    ///  "properties": {
    ///    "rootUserIds": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "subOrganizationId": {
    ///      "type": "string"
    ///    },
    ///    "wallet": {
    ///      "$ref": "#/components/schemas/WalletResult"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateSubOrganizationResultV5 {
        #[serde(rename = "rootUserIds", default, skip_serializing_if = "Vec::is_empty")]
        pub root_user_ids: Vec<String>,
        #[serde(rename = "subOrganizationId")]
        pub sub_organization_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub wallet: Option<WalletResult>,
    }

    impl From<&CreateSubOrganizationResultV5> for CreateSubOrganizationResultV5 {
        fn from(value: &CreateSubOrganizationResultV5) -> Self {
            value.clone()
        }
    }

    ///CreateSubOrganizationResultV6
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "subOrganizationId"
    ///  ],
    ///  "properties": {
    ///    "rootUserIds": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "subOrganizationId": {
    ///      "type": "string"
    ///    },
    ///    "wallet": {
    ///      "$ref": "#/components/schemas/WalletResult"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateSubOrganizationResultV6 {
        #[serde(rename = "rootUserIds", default, skip_serializing_if = "Vec::is_empty")]
        pub root_user_ids: Vec<String>,
        #[serde(rename = "subOrganizationId")]
        pub sub_organization_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub wallet: Option<WalletResult>,
    }

    impl From<&CreateSubOrganizationResultV6> for CreateSubOrganizationResultV6 {
        fn from(value: &CreateSubOrganizationResultV6) -> Self {
            value.clone()
        }
    }

    ///CreateSubOrganizationResultV7
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "subOrganizationId"
    ///  ],
    ///  "properties": {
    ///    "rootUserIds": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "subOrganizationId": {
    ///      "type": "string"
    ///    },
    ///    "wallet": {
    ///      "$ref": "#/components/schemas/WalletResult"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateSubOrganizationResultV7 {
        #[serde(rename = "rootUserIds", default, skip_serializing_if = "Vec::is_empty")]
        pub root_user_ids: Vec<String>,
        #[serde(rename = "subOrganizationId")]
        pub sub_organization_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub wallet: Option<WalletResult>,
    }

    impl From<&CreateSubOrganizationResultV7> for CreateSubOrganizationResultV7 {
        fn from(value: &CreateSubOrganizationResultV7) -> Self {
            value.clone()
        }
    }

    ///CreateUserTagIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "userIds",
    ///    "userTagName"
    ///  ],
    ///  "properties": {
    ///    "userIds": {
    ///      "description": "A list of User IDs.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "userTagName": {
    ///      "description": "Human-readable name for a User Tag.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateUserTagIntent {
        ///A list of User IDs.
        #[serde(rename = "userIds")]
        pub user_ids: Vec<String>,
        ///Human-readable name for a User Tag.
        #[serde(rename = "userTagName")]
        pub user_tag_name: String,
    }

    impl From<&CreateUserTagIntent> for CreateUserTagIntent {
        fn from(value: &CreateUserTagIntent) -> Self {
            value.clone()
        }
    }

    ///CreateUserTagRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/CreateUserTagIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_CREATE_USER_TAG"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateUserTagRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: CreateUserTagIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: CreateUserTagRequestType,
    }

    impl From<&CreateUserTagRequest> for CreateUserTagRequest {
        fn from(value: &CreateUserTagRequest) -> Self {
            value.clone()
        }
    }

    ///CreateUserTagRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_CREATE_USER_TAG"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum CreateUserTagRequestType {
        #[serde(rename = "ACTIVITY_TYPE_CREATE_USER_TAG")]
        ActivityTypeCreateUserTag,
    }

    impl From<&CreateUserTagRequestType> for CreateUserTagRequestType {
        fn from(value: &CreateUserTagRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for CreateUserTagRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeCreateUserTag => write!(f, "ACTIVITY_TYPE_CREATE_USER_TAG"),
            }
        }
    }

    impl std::str::FromStr for CreateUserTagRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_CREATE_USER_TAG" => Ok(Self::ActivityTypeCreateUserTag),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for CreateUserTagRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for CreateUserTagRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for CreateUserTagRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///CreateUserTagResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "userIds",
    ///    "userTagId"
    ///  ],
    ///  "properties": {
    ///    "userIds": {
    ///      "description": "A list of User IDs.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "userTagId": {
    ///      "description": "Unique identifier for a given User Tag.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateUserTagResult {
        ///A list of User IDs.
        #[serde(rename = "userIds")]
        pub user_ids: Vec<String>,
        ///Unique identifier for a given User Tag.
        #[serde(rename = "userTagId")]
        pub user_tag_id: String,
    }

    impl From<&CreateUserTagResult> for CreateUserTagResult {
        fn from(value: &CreateUserTagResult) -> Self {
            value.clone()
        }
    }

    ///CreateUsersIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "users"
    ///  ],
    ///  "properties": {
    ///    "users": {
    ///      "description": "A list of Users.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/UserParams"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateUsersIntent {
        ///A list of Users.
        pub users: Vec<UserParams>,
    }

    impl From<&CreateUsersIntent> for CreateUsersIntent {
        fn from(value: &CreateUsersIntent) -> Self {
            value.clone()
        }
    }

    ///CreateUsersIntentV2
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "users"
    ///  ],
    ///  "properties": {
    ///    "users": {
    ///      "description": "A list of Users.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/UserParamsV2"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateUsersIntentV2 {
        ///A list of Users.
        pub users: Vec<UserParamsV2>,
    }

    impl From<&CreateUsersIntentV2> for CreateUsersIntentV2 {
        fn from(value: &CreateUsersIntentV2) -> Self {
            value.clone()
        }
    }

    ///CreateUsersRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/CreateUsersIntentV2"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_CREATE_USERS_V2"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateUsersRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: CreateUsersIntentV2,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: CreateUsersRequestType,
    }

    impl From<&CreateUsersRequest> for CreateUsersRequest {
        fn from(value: &CreateUsersRequest) -> Self {
            value.clone()
        }
    }

    ///CreateUsersRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_CREATE_USERS_V2"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum CreateUsersRequestType {
        #[serde(rename = "ACTIVITY_TYPE_CREATE_USERS_V2")]
        ActivityTypeCreateUsersV2,
    }

    impl From<&CreateUsersRequestType> for CreateUsersRequestType {
        fn from(value: &CreateUsersRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for CreateUsersRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeCreateUsersV2 => write!(f, "ACTIVITY_TYPE_CREATE_USERS_V2"),
            }
        }
    }

    impl std::str::FromStr for CreateUsersRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_CREATE_USERS_V2" => Ok(Self::ActivityTypeCreateUsersV2),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for CreateUsersRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for CreateUsersRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for CreateUsersRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///CreateUsersResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "userIds"
    ///  ],
    ///  "properties": {
    ///    "userIds": {
    ///      "description": "A list of User IDs.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateUsersResult {
        ///A list of User IDs.
        #[serde(rename = "userIds")]
        pub user_ids: Vec<String>,
    }

    impl From<&CreateUsersResult> for CreateUsersResult {
        fn from(value: &CreateUsersResult) -> Self {
            value.clone()
        }
    }

    ///CreateWalletAccountsIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "accounts",
    ///    "walletId"
    ///  ],
    ///  "properties": {
    ///    "accounts": {
    ///      "description": "A list of wallet Accounts.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/WalletAccountParams"
    ///      }
    ///    },
    ///    "walletId": {
    ///      "description": "Unique identifier for a given Wallet.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateWalletAccountsIntent {
        ///A list of wallet Accounts.
        pub accounts: Vec<WalletAccountParams>,
        ///Unique identifier for a given Wallet.
        #[serde(rename = "walletId")]
        pub wallet_id: String,
    }

    impl From<&CreateWalletAccountsIntent> for CreateWalletAccountsIntent {
        fn from(value: &CreateWalletAccountsIntent) -> Self {
            value.clone()
        }
    }

    ///CreateWalletAccountsRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/CreateWalletAccountsIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_CREATE_WALLET_ACCOUNTS"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateWalletAccountsRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: CreateWalletAccountsIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: CreateWalletAccountsRequestType,
    }

    impl From<&CreateWalletAccountsRequest> for CreateWalletAccountsRequest {
        fn from(value: &CreateWalletAccountsRequest) -> Self {
            value.clone()
        }
    }

    ///CreateWalletAccountsRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_CREATE_WALLET_ACCOUNTS"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum CreateWalletAccountsRequestType {
        #[serde(rename = "ACTIVITY_TYPE_CREATE_WALLET_ACCOUNTS")]
        ActivityTypeCreateWalletAccounts,
    }

    impl From<&CreateWalletAccountsRequestType> for CreateWalletAccountsRequestType {
        fn from(value: &CreateWalletAccountsRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for CreateWalletAccountsRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeCreateWalletAccounts => {
                    write!(f, "ACTIVITY_TYPE_CREATE_WALLET_ACCOUNTS")
                }
            }
        }
    }

    impl std::str::FromStr for CreateWalletAccountsRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_CREATE_WALLET_ACCOUNTS" => {
                    Ok(Self::ActivityTypeCreateWalletAccounts)
                }
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for CreateWalletAccountsRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for CreateWalletAccountsRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for CreateWalletAccountsRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///CreateWalletAccountsResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "addresses"
    ///  ],
    ///  "properties": {
    ///    "addresses": {
    ///      "description": "A list of derived addresses.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateWalletAccountsResult {
        ///A list of derived addresses.
        pub addresses: Vec<String>,
    }

    impl From<&CreateWalletAccountsResult> for CreateWalletAccountsResult {
        fn from(value: &CreateWalletAccountsResult) -> Self {
            value.clone()
        }
    }

    ///CreateWalletIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "accounts",
    ///    "walletName"
    ///  ],
    ///  "properties": {
    ///    "accounts": {
    ///      "description": "A list of wallet Accounts.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/WalletAccountParams"
    ///      }
    ///    },
    ///    "mnemonicLength": {
    ///      "description": "Length of mnemonic to generate the Wallet seed.
    /// Defaults to 12. Accepted values: 12, 15, 18, 21, 24.",
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "walletName": {
    ///      "description": "Human-readable name for a Wallet.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateWalletIntent {
        ///A list of wallet Accounts.
        pub accounts: Vec<WalletAccountParams>,
        ///Length of mnemonic to generate the Wallet seed. Defaults to 12.
        /// Accepted values: 12, 15, 18, 21, 24.
        #[serde(
            rename = "mnemonicLength",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub mnemonic_length: Option<i32>,
        ///Human-readable name for a Wallet.
        #[serde(rename = "walletName")]
        pub wallet_name: String,
    }

    impl From<&CreateWalletIntent> for CreateWalletIntent {
        fn from(value: &CreateWalletIntent) -> Self {
            value.clone()
        }
    }

    ///CreateWalletRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/CreateWalletIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_CREATE_WALLET"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateWalletRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: CreateWalletIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: CreateWalletRequestType,
    }

    impl From<&CreateWalletRequest> for CreateWalletRequest {
        fn from(value: &CreateWalletRequest) -> Self {
            value.clone()
        }
    }

    ///CreateWalletRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_CREATE_WALLET"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum CreateWalletRequestType {
        #[serde(rename = "ACTIVITY_TYPE_CREATE_WALLET")]
        ActivityTypeCreateWallet,
    }

    impl From<&CreateWalletRequestType> for CreateWalletRequestType {
        fn from(value: &CreateWalletRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for CreateWalletRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeCreateWallet => write!(f, "ACTIVITY_TYPE_CREATE_WALLET"),
            }
        }
    }

    impl std::str::FromStr for CreateWalletRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_CREATE_WALLET" => Ok(Self::ActivityTypeCreateWallet),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for CreateWalletRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for CreateWalletRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for CreateWalletRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///CreateWalletResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "addresses",
    ///    "walletId"
    ///  ],
    ///  "properties": {
    ///    "addresses": {
    ///      "description": "A list of account addresses.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "walletId": {
    ///      "description": "Unique identifier for a Wallet.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CreateWalletResult {
        ///A list of account addresses.
        pub addresses: Vec<String>,
        ///Unique identifier for a Wallet.
        #[serde(rename = "walletId")]
        pub wallet_id: String,
    }

    impl From<&CreateWalletResult> for CreateWalletResult {
        fn from(value: &CreateWalletResult) -> Self {
            value.clone()
        }
    }

    ///CredPropsAuthenticationExtensionsClientOutputs
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "rk"
    ///  ],
    ///  "properties": {
    ///    "rk": {
    ///      "type": "boolean"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CredPropsAuthenticationExtensionsClientOutputs {
        pub rk: bool,
    }

    impl From<&CredPropsAuthenticationExtensionsClientOutputs>
        for CredPropsAuthenticationExtensionsClientOutputs
    {
        fn from(value: &CredPropsAuthenticationExtensionsClientOutputs) -> Self {
            value.clone()
        }
    }

    ///CredentialType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "CREDENTIAL_TYPE_WEBAUTHN_AUTHENTICATOR",
    ///    "CREDENTIAL_TYPE_API_KEY_P256",
    ///    "CREDENTIAL_TYPE_RECOVER_USER_KEY_P256",
    ///    "CREDENTIAL_TYPE_API_KEY_SECP256K1",
    ///    "CREDENTIAL_TYPE_EMAIL_AUTH_KEY_P256",
    ///    "CREDENTIAL_TYPE_API_KEY_ED25519",
    ///    "CREDENTIAL_TYPE_OTP_AUTH_KEY_P256"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum CredentialType {
        #[serde(rename = "CREDENTIAL_TYPE_WEBAUTHN_AUTHENTICATOR")]
        CredentialTypeWebauthnAuthenticator,
        #[serde(rename = "CREDENTIAL_TYPE_API_KEY_P256")]
        CredentialTypeApiKeyP256,
        #[serde(rename = "CREDENTIAL_TYPE_RECOVER_USER_KEY_P256")]
        CredentialTypeRecoverUserKeyP256,
        #[serde(rename = "CREDENTIAL_TYPE_API_KEY_SECP256K1")]
        CredentialTypeApiKeySecp256k1,
        #[serde(rename = "CREDENTIAL_TYPE_EMAIL_AUTH_KEY_P256")]
        CredentialTypeEmailAuthKeyP256,
        #[serde(rename = "CREDENTIAL_TYPE_API_KEY_ED25519")]
        CredentialTypeApiKeyEd25519,
        #[serde(rename = "CREDENTIAL_TYPE_OTP_AUTH_KEY_P256")]
        CredentialTypeOtpAuthKeyP256,
    }

    impl From<&CredentialType> for CredentialType {
        fn from(value: &CredentialType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for CredentialType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::CredentialTypeWebauthnAuthenticator => {
                    write!(f, "CREDENTIAL_TYPE_WEBAUTHN_AUTHENTICATOR")
                }
                Self::CredentialTypeApiKeyP256 => write!(f, "CREDENTIAL_TYPE_API_KEY_P256"),
                Self::CredentialTypeRecoverUserKeyP256 => {
                    write!(f, "CREDENTIAL_TYPE_RECOVER_USER_KEY_P256")
                }
                Self::CredentialTypeApiKeySecp256k1 => {
                    write!(f, "CREDENTIAL_TYPE_API_KEY_SECP256K1")
                }
                Self::CredentialTypeEmailAuthKeyP256 => {
                    write!(f, "CREDENTIAL_TYPE_EMAIL_AUTH_KEY_P256")
                }
                Self::CredentialTypeApiKeyEd25519 => write!(f, "CREDENTIAL_TYPE_API_KEY_ED25519"),
                Self::CredentialTypeOtpAuthKeyP256 => {
                    write!(f, "CREDENTIAL_TYPE_OTP_AUTH_KEY_P256")
                }
            }
        }
    }

    impl std::str::FromStr for CredentialType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "CREDENTIAL_TYPE_WEBAUTHN_AUTHENTICATOR" => {
                    Ok(Self::CredentialTypeWebauthnAuthenticator)
                }
                "CREDENTIAL_TYPE_API_KEY_P256" => Ok(Self::CredentialTypeApiKeyP256),
                "CREDENTIAL_TYPE_RECOVER_USER_KEY_P256" => {
                    Ok(Self::CredentialTypeRecoverUserKeyP256)
                }
                "CREDENTIAL_TYPE_API_KEY_SECP256K1" => Ok(Self::CredentialTypeApiKeySecp256k1),
                "CREDENTIAL_TYPE_EMAIL_AUTH_KEY_P256" => Ok(Self::CredentialTypeEmailAuthKeyP256),
                "CREDENTIAL_TYPE_API_KEY_ED25519" => Ok(Self::CredentialTypeApiKeyEd25519),
                "CREDENTIAL_TYPE_OTP_AUTH_KEY_P256" => Ok(Self::CredentialTypeOtpAuthKeyP256),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for CredentialType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for CredentialType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for CredentialType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///Curve
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "CURVE_SECP256K1",
    ///    "CURVE_ED25519"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum Curve {
        #[serde(rename = "CURVE_SECP256K1")]
        CurveSecp256k1,
        #[serde(rename = "CURVE_ED25519")]
        CurveEd25519,
    }

    impl From<&Curve> for Curve {
        fn from(value: &Curve) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for Curve {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::CurveSecp256k1 => write!(f, "CURVE_SECP256K1"),
                Self::CurveEd25519 => write!(f, "CURVE_ED25519"),
            }
        }
    }

    impl std::str::FromStr for Curve {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "CURVE_SECP256K1" => Ok(Self::CurveSecp256k1),
                "CURVE_ED25519" => Ok(Self::CurveEd25519),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for Curve {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for Curve {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for Curve {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///DataV1Address
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "address": {
    ///      "type": "string"
    ///    },
    ///    "format": {
    ///      "$ref": "#/components/schemas/AddressFormat"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DataV1Address {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub address: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub format: Option<AddressFormat>,
    }

    impl From<&DataV1Address> for DataV1Address {
        fn from(value: &DataV1Address) -> Self {
            value.clone()
        }
    }

    ///DeleteApiKeysIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "apiKeyIds",
    ///    "userId"
    ///  ],
    ///  "properties": {
    ///    "apiKeyIds": {
    ///      "description": "A list of API Key IDs.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "userId": {
    ///      "description": "Unique identifier for a given User.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeleteApiKeysIntent {
        ///A list of API Key IDs.
        #[serde(rename = "apiKeyIds")]
        pub api_key_ids: Vec<String>,
        ///Unique identifier for a given User.
        #[serde(rename = "userId")]
        pub user_id: String,
    }

    impl From<&DeleteApiKeysIntent> for DeleteApiKeysIntent {
        fn from(value: &DeleteApiKeysIntent) -> Self {
            value.clone()
        }
    }

    ///DeleteApiKeysRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/DeleteApiKeysIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_DELETE_API_KEYS"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeleteApiKeysRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: DeleteApiKeysIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: DeleteApiKeysRequestType,
    }

    impl From<&DeleteApiKeysRequest> for DeleteApiKeysRequest {
        fn from(value: &DeleteApiKeysRequest) -> Self {
            value.clone()
        }
    }

    ///DeleteApiKeysRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_DELETE_API_KEYS"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum DeleteApiKeysRequestType {
        #[serde(rename = "ACTIVITY_TYPE_DELETE_API_KEYS")]
        ActivityTypeDeleteApiKeys,
    }

    impl From<&DeleteApiKeysRequestType> for DeleteApiKeysRequestType {
        fn from(value: &DeleteApiKeysRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for DeleteApiKeysRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeDeleteApiKeys => write!(f, "ACTIVITY_TYPE_DELETE_API_KEYS"),
            }
        }
    }

    impl std::str::FromStr for DeleteApiKeysRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_DELETE_API_KEYS" => Ok(Self::ActivityTypeDeleteApiKeys),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for DeleteApiKeysRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for DeleteApiKeysRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for DeleteApiKeysRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///DeleteApiKeysResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "apiKeyIds"
    ///  ],
    ///  "properties": {
    ///    "apiKeyIds": {
    ///      "description": "A list of API Key IDs.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeleteApiKeysResult {
        ///A list of API Key IDs.
        #[serde(rename = "apiKeyIds")]
        pub api_key_ids: Vec<String>,
    }

    impl From<&DeleteApiKeysResult> for DeleteApiKeysResult {
        fn from(value: &DeleteApiKeysResult) -> Self {
            value.clone()
        }
    }

    ///DeleteAuthenticatorsIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "authenticatorIds",
    ///    "userId"
    ///  ],
    ///  "properties": {
    ///    "authenticatorIds": {
    ///      "description": "A list of Authenticator IDs.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "userId": {
    ///      "description": "Unique identifier for a given User.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeleteAuthenticatorsIntent {
        ///A list of Authenticator IDs.
        #[serde(rename = "authenticatorIds")]
        pub authenticator_ids: Vec<String>,
        ///Unique identifier for a given User.
        #[serde(rename = "userId")]
        pub user_id: String,
    }

    impl From<&DeleteAuthenticatorsIntent> for DeleteAuthenticatorsIntent {
        fn from(value: &DeleteAuthenticatorsIntent) -> Self {
            value.clone()
        }
    }

    ///DeleteAuthenticatorsRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/DeleteAuthenticatorsIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_DELETE_AUTHENTICATORS"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeleteAuthenticatorsRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: DeleteAuthenticatorsIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: DeleteAuthenticatorsRequestType,
    }

    impl From<&DeleteAuthenticatorsRequest> for DeleteAuthenticatorsRequest {
        fn from(value: &DeleteAuthenticatorsRequest) -> Self {
            value.clone()
        }
    }

    ///DeleteAuthenticatorsRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_DELETE_AUTHENTICATORS"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum DeleteAuthenticatorsRequestType {
        #[serde(rename = "ACTIVITY_TYPE_DELETE_AUTHENTICATORS")]
        ActivityTypeDeleteAuthenticators,
    }

    impl From<&DeleteAuthenticatorsRequestType> for DeleteAuthenticatorsRequestType {
        fn from(value: &DeleteAuthenticatorsRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for DeleteAuthenticatorsRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeDeleteAuthenticators => {
                    write!(f, "ACTIVITY_TYPE_DELETE_AUTHENTICATORS")
                }
            }
        }
    }

    impl std::str::FromStr for DeleteAuthenticatorsRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_DELETE_AUTHENTICATORS" => Ok(Self::ActivityTypeDeleteAuthenticators),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for DeleteAuthenticatorsRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for DeleteAuthenticatorsRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for DeleteAuthenticatorsRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///DeleteAuthenticatorsResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "authenticatorIds"
    ///  ],
    ///  "properties": {
    ///    "authenticatorIds": {
    ///      "description": "Unique identifier for a given Authenticator.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeleteAuthenticatorsResult {
        ///Unique identifier for a given Authenticator.
        #[serde(rename = "authenticatorIds")]
        pub authenticator_ids: Vec<String>,
    }

    impl From<&DeleteAuthenticatorsResult> for DeleteAuthenticatorsResult {
        fn from(value: &DeleteAuthenticatorsResult) -> Self {
            value.clone()
        }
    }

    ///DeleteInvitationIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "invitationId"
    ///  ],
    ///  "properties": {
    ///    "invitationId": {
    ///      "description": "Unique identifier for a given Invitation object.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeleteInvitationIntent {
        ///Unique identifier for a given Invitation object.
        #[serde(rename = "invitationId")]
        pub invitation_id: String,
    }

    impl From<&DeleteInvitationIntent> for DeleteInvitationIntent {
        fn from(value: &DeleteInvitationIntent) -> Self {
            value.clone()
        }
    }

    ///DeleteInvitationRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/DeleteInvitationIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_DELETE_INVITATION"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeleteInvitationRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: DeleteInvitationIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: DeleteInvitationRequestType,
    }

    impl From<&DeleteInvitationRequest> for DeleteInvitationRequest {
        fn from(value: &DeleteInvitationRequest) -> Self {
            value.clone()
        }
    }

    ///DeleteInvitationRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_DELETE_INVITATION"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum DeleteInvitationRequestType {
        #[serde(rename = "ACTIVITY_TYPE_DELETE_INVITATION")]
        ActivityTypeDeleteInvitation,
    }

    impl From<&DeleteInvitationRequestType> for DeleteInvitationRequestType {
        fn from(value: &DeleteInvitationRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for DeleteInvitationRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeDeleteInvitation => write!(f, "ACTIVITY_TYPE_DELETE_INVITATION"),
            }
        }
    }

    impl std::str::FromStr for DeleteInvitationRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_DELETE_INVITATION" => Ok(Self::ActivityTypeDeleteInvitation),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for DeleteInvitationRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for DeleteInvitationRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for DeleteInvitationRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///DeleteInvitationResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "invitationId"
    ///  ],
    ///  "properties": {
    ///    "invitationId": {
    ///      "description": "Unique identifier for a given Invitation.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeleteInvitationResult {
        ///Unique identifier for a given Invitation.
        #[serde(rename = "invitationId")]
        pub invitation_id: String,
    }

    impl From<&DeleteInvitationResult> for DeleteInvitationResult {
        fn from(value: &DeleteInvitationResult) -> Self {
            value.clone()
        }
    }

    ///DeleteOauthProvidersIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "providerIds",
    ///    "userId"
    ///  ],
    ///  "properties": {
    ///    "providerIds": {
    ///      "description": "Unique identifier for a given Provider.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "userId": {
    ///      "description": "The ID of the User to remove an Oauth provider
    /// from",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeleteOauthProvidersIntent {
        ///Unique identifier for a given Provider.
        #[serde(rename = "providerIds")]
        pub provider_ids: Vec<String>,
        ///The ID of the User to remove an Oauth provider from
        #[serde(rename = "userId")]
        pub user_id: String,
    }

    impl From<&DeleteOauthProvidersIntent> for DeleteOauthProvidersIntent {
        fn from(value: &DeleteOauthProvidersIntent) -> Self {
            value.clone()
        }
    }

    ///DeleteOauthProvidersRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/DeleteOauthProvidersIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_DELETE_OAUTH_PROVIDERS"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeleteOauthProvidersRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: DeleteOauthProvidersIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: DeleteOauthProvidersRequestType,
    }

    impl From<&DeleteOauthProvidersRequest> for DeleteOauthProvidersRequest {
        fn from(value: &DeleteOauthProvidersRequest) -> Self {
            value.clone()
        }
    }

    ///DeleteOauthProvidersRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_DELETE_OAUTH_PROVIDERS"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum DeleteOauthProvidersRequestType {
        #[serde(rename = "ACTIVITY_TYPE_DELETE_OAUTH_PROVIDERS")]
        ActivityTypeDeleteOauthProviders,
    }

    impl From<&DeleteOauthProvidersRequestType> for DeleteOauthProvidersRequestType {
        fn from(value: &DeleteOauthProvidersRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for DeleteOauthProvidersRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeDeleteOauthProviders => {
                    write!(f, "ACTIVITY_TYPE_DELETE_OAUTH_PROVIDERS")
                }
            }
        }
    }

    impl std::str::FromStr for DeleteOauthProvidersRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_DELETE_OAUTH_PROVIDERS" => {
                    Ok(Self::ActivityTypeDeleteOauthProviders)
                }
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for DeleteOauthProvidersRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for DeleteOauthProvidersRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for DeleteOauthProvidersRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///DeleteOauthProvidersResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "providerIds"
    ///  ],
    ///  "properties": {
    ///    "providerIds": {
    ///      "description": "A list of unique identifiers for Oauth Providers",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeleteOauthProvidersResult {
        ///A list of unique identifiers for Oauth Providers
        #[serde(rename = "providerIds")]
        pub provider_ids: Vec<String>,
    }

    impl From<&DeleteOauthProvidersResult> for DeleteOauthProvidersResult {
        fn from(value: &DeleteOauthProvidersResult) -> Self {
            value.clone()
        }
    }

    ///DeleteOrganizationIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeleteOrganizationIntent {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
    }

    impl From<&DeleteOrganizationIntent> for DeleteOrganizationIntent {
        fn from(value: &DeleteOrganizationIntent) -> Self {
            value.clone()
        }
    }

    ///DeleteOrganizationResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeleteOrganizationResult {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
    }

    impl From<&DeleteOrganizationResult> for DeleteOrganizationResult {
        fn from(value: &DeleteOrganizationResult) -> Self {
            value.clone()
        }
    }

    ///DeletePaymentMethodIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "paymentMethodId"
    ///  ],
    ///  "properties": {
    ///    "paymentMethodId": {
    ///      "description": "The payment method that the customer wants to
    /// remove.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeletePaymentMethodIntent {
        ///The payment method that the customer wants to remove.
        #[serde(rename = "paymentMethodId")]
        pub payment_method_id: String,
    }

    impl From<&DeletePaymentMethodIntent> for DeletePaymentMethodIntent {
        fn from(value: &DeletePaymentMethodIntent) -> Self {
            value.clone()
        }
    }

    ///DeletePaymentMethodResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "paymentMethodId"
    ///  ],
    ///  "properties": {
    ///    "paymentMethodId": {
    ///      "description": "The payment method that was removed.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeletePaymentMethodResult {
        ///The payment method that was removed.
        #[serde(rename = "paymentMethodId")]
        pub payment_method_id: String,
    }

    impl From<&DeletePaymentMethodResult> for DeletePaymentMethodResult {
        fn from(value: &DeletePaymentMethodResult) -> Self {
            value.clone()
        }
    }

    ///DeletePolicyIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "policyId"
    ///  ],
    ///  "properties": {
    ///    "policyId": {
    ///      "description": "Unique identifier for a given Policy.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeletePolicyIntent {
        ///Unique identifier for a given Policy.
        #[serde(rename = "policyId")]
        pub policy_id: String,
    }

    impl From<&DeletePolicyIntent> for DeletePolicyIntent {
        fn from(value: &DeletePolicyIntent) -> Self {
            value.clone()
        }
    }

    ///DeletePolicyRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/DeletePolicyIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_DELETE_POLICY"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeletePolicyRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: DeletePolicyIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: DeletePolicyRequestType,
    }

    impl From<&DeletePolicyRequest> for DeletePolicyRequest {
        fn from(value: &DeletePolicyRequest) -> Self {
            value.clone()
        }
    }

    ///DeletePolicyRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_DELETE_POLICY"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum DeletePolicyRequestType {
        #[serde(rename = "ACTIVITY_TYPE_DELETE_POLICY")]
        ActivityTypeDeletePolicy,
    }

    impl From<&DeletePolicyRequestType> for DeletePolicyRequestType {
        fn from(value: &DeletePolicyRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for DeletePolicyRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeDeletePolicy => write!(f, "ACTIVITY_TYPE_DELETE_POLICY"),
            }
        }
    }

    impl std::str::FromStr for DeletePolicyRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_DELETE_POLICY" => Ok(Self::ActivityTypeDeletePolicy),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for DeletePolicyRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for DeletePolicyRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for DeletePolicyRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///DeletePolicyResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "policyId"
    ///  ],
    ///  "properties": {
    ///    "policyId": {
    ///      "description": "Unique identifier for a given Policy.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeletePolicyResult {
        ///Unique identifier for a given Policy.
        #[serde(rename = "policyId")]
        pub policy_id: String,
    }

    impl From<&DeletePolicyResult> for DeletePolicyResult {
        fn from(value: &DeletePolicyResult) -> Self {
            value.clone()
        }
    }

    ///DeletePrivateKeyTagsIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "privateKeyTagIds"
    ///  ],
    ///  "properties": {
    ///    "privateKeyTagIds": {
    ///      "description": "A list of Private Key Tag IDs.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeletePrivateKeyTagsIntent {
        ///A list of Private Key Tag IDs.
        #[serde(rename = "privateKeyTagIds")]
        pub private_key_tag_ids: Vec<String>,
    }

    impl From<&DeletePrivateKeyTagsIntent> for DeletePrivateKeyTagsIntent {
        fn from(value: &DeletePrivateKeyTagsIntent) -> Self {
            value.clone()
        }
    }

    ///DeletePrivateKeyTagsRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/DeletePrivateKeyTagsIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_DELETE_PRIVATE_KEY_TAGS"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeletePrivateKeyTagsRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: DeletePrivateKeyTagsIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: DeletePrivateKeyTagsRequestType,
    }

    impl From<&DeletePrivateKeyTagsRequest> for DeletePrivateKeyTagsRequest {
        fn from(value: &DeletePrivateKeyTagsRequest) -> Self {
            value.clone()
        }
    }

    ///DeletePrivateKeyTagsRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_DELETE_PRIVATE_KEY_TAGS"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum DeletePrivateKeyTagsRequestType {
        #[serde(rename = "ACTIVITY_TYPE_DELETE_PRIVATE_KEY_TAGS")]
        ActivityTypeDeletePrivateKeyTags,
    }

    impl From<&DeletePrivateKeyTagsRequestType> for DeletePrivateKeyTagsRequestType {
        fn from(value: &DeletePrivateKeyTagsRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for DeletePrivateKeyTagsRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeDeletePrivateKeyTags => {
                    write!(f, "ACTIVITY_TYPE_DELETE_PRIVATE_KEY_TAGS")
                }
            }
        }
    }

    impl std::str::FromStr for DeletePrivateKeyTagsRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_DELETE_PRIVATE_KEY_TAGS" => {
                    Ok(Self::ActivityTypeDeletePrivateKeyTags)
                }
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for DeletePrivateKeyTagsRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for DeletePrivateKeyTagsRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for DeletePrivateKeyTagsRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///DeletePrivateKeyTagsResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "privateKeyIds",
    ///    "privateKeyTagIds"
    ///  ],
    ///  "properties": {
    ///    "privateKeyIds": {
    ///      "description": "A list of Private Key IDs.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "privateKeyTagIds": {
    ///      "description": "A list of Private Key Tag IDs.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeletePrivateKeyTagsResult {
        ///A list of Private Key IDs.
        #[serde(rename = "privateKeyIds")]
        pub private_key_ids: Vec<String>,
        ///A list of Private Key Tag IDs.
        #[serde(rename = "privateKeyTagIds")]
        pub private_key_tag_ids: Vec<String>,
    }

    impl From<&DeletePrivateKeyTagsResult> for DeletePrivateKeyTagsResult {
        fn from(value: &DeletePrivateKeyTagsResult) -> Self {
            value.clone()
        }
    }

    ///DeletePrivateKeysIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "privateKeyIds"
    ///  ],
    ///  "properties": {
    ///    "deleteWithoutExport": {
    ///      "description": "Optional parameter for deleting the private keys,
    /// even if any have not been previously exported. If they have been
    /// exported, this field is ignored.",
    ///      "type": "boolean"
    ///    },
    ///    "privateKeyIds": {
    ///      "description": "List of unique identifiers for private keys within
    /// an organization",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeletePrivateKeysIntent {
        ///Optional parameter for deleting the private keys, even if any have
        /// not been previously exported. If they have been exported, this field
        /// is ignored.
        #[serde(
            rename = "deleteWithoutExport",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_without_export: Option<bool>,
        ///List of unique identifiers for private keys within an organization
        #[serde(rename = "privateKeyIds")]
        pub private_key_ids: Vec<String>,
    }

    impl From<&DeletePrivateKeysIntent> for DeletePrivateKeysIntent {
        fn from(value: &DeletePrivateKeysIntent) -> Self {
            value.clone()
        }
    }

    ///DeletePrivateKeysRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/DeletePrivateKeysIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_DELETE_PRIVATE_KEYS"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeletePrivateKeysRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: DeletePrivateKeysIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: DeletePrivateKeysRequestType,
    }

    impl From<&DeletePrivateKeysRequest> for DeletePrivateKeysRequest {
        fn from(value: &DeletePrivateKeysRequest) -> Self {
            value.clone()
        }
    }

    ///DeletePrivateKeysRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_DELETE_PRIVATE_KEYS"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum DeletePrivateKeysRequestType {
        #[serde(rename = "ACTIVITY_TYPE_DELETE_PRIVATE_KEYS")]
        ActivityTypeDeletePrivateKeys,
    }

    impl From<&DeletePrivateKeysRequestType> for DeletePrivateKeysRequestType {
        fn from(value: &DeletePrivateKeysRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for DeletePrivateKeysRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeDeletePrivateKeys => {
                    write!(f, "ACTIVITY_TYPE_DELETE_PRIVATE_KEYS")
                }
            }
        }
    }

    impl std::str::FromStr for DeletePrivateKeysRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_DELETE_PRIVATE_KEYS" => Ok(Self::ActivityTypeDeletePrivateKeys),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for DeletePrivateKeysRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for DeletePrivateKeysRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for DeletePrivateKeysRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///DeletePrivateKeysResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "privateKeyIds"
    ///  ],
    ///  "properties": {
    ///    "privateKeyIds": {
    ///      "description": "A list of private key unique identifiers that were
    /// removed",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeletePrivateKeysResult {
        ///A list of private key unique identifiers that were removed
        #[serde(rename = "privateKeyIds")]
        pub private_key_ids: Vec<String>,
    }

    impl From<&DeletePrivateKeysResult> for DeletePrivateKeysResult {
        fn from(value: &DeletePrivateKeysResult) -> Self {
            value.clone()
        }
    }

    ///DeleteSubOrganizationIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "deleteWithoutExport": {
    ///      "description": "Sub-organization deletion, by default, requires
    /// associated wallets and private keys to be exported for security reasons.
    /// Set this boolean to true to force sub-organization deletion even if some
    /// wallets or private keys within it have not been exported yet. Default:
    /// false.",
    ///      "type": "boolean"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeleteSubOrganizationIntent {
        ///Sub-organization deletion, by default, requires associated wallets
        /// and private keys to be exported for security reasons. Set this
        /// boolean to true to force sub-organization deletion even if some
        /// wallets or private keys within it have not been exported yet.
        /// Default: false.
        #[serde(
            rename = "deleteWithoutExport",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_without_export: Option<bool>,
    }

    impl From<&DeleteSubOrganizationIntent> for DeleteSubOrganizationIntent {
        fn from(value: &DeleteSubOrganizationIntent) -> Self {
            value.clone()
        }
    }

    ///DeleteSubOrganizationRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/DeleteSubOrganizationIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_DELETE_SUB_ORGANIZATION"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeleteSubOrganizationRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: DeleteSubOrganizationIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: DeleteSubOrganizationRequestType,
    }

    impl From<&DeleteSubOrganizationRequest> for DeleteSubOrganizationRequest {
        fn from(value: &DeleteSubOrganizationRequest) -> Self {
            value.clone()
        }
    }

    ///DeleteSubOrganizationRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_DELETE_SUB_ORGANIZATION"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum DeleteSubOrganizationRequestType {
        #[serde(rename = "ACTIVITY_TYPE_DELETE_SUB_ORGANIZATION")]
        ActivityTypeDeleteSubOrganization,
    }

    impl From<&DeleteSubOrganizationRequestType> for DeleteSubOrganizationRequestType {
        fn from(value: &DeleteSubOrganizationRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for DeleteSubOrganizationRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeDeleteSubOrganization => {
                    write!(f, "ACTIVITY_TYPE_DELETE_SUB_ORGANIZATION")
                }
            }
        }
    }

    impl std::str::FromStr for DeleteSubOrganizationRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_DELETE_SUB_ORGANIZATION" => {
                    Ok(Self::ActivityTypeDeleteSubOrganization)
                }
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for DeleteSubOrganizationRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for DeleteSubOrganizationRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for DeleteSubOrganizationRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///DeleteSubOrganizationResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "subOrganizationUuid"
    ///  ],
    ///  "properties": {
    ///    "subOrganizationUuid": {
    ///      "description": "Unique identifier of the sub organization that was
    /// removed",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeleteSubOrganizationResult {
        ///Unique identifier of the sub organization that was removed
        #[serde(rename = "subOrganizationUuid")]
        pub sub_organization_uuid: String,
    }

    impl From<&DeleteSubOrganizationResult> for DeleteSubOrganizationResult {
        fn from(value: &DeleteSubOrganizationResult) -> Self {
            value.clone()
        }
    }

    ///DeleteUserTagsIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "userTagIds"
    ///  ],
    ///  "properties": {
    ///    "userTagIds": {
    ///      "description": "A list of User Tag IDs.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeleteUserTagsIntent {
        ///A list of User Tag IDs.
        #[serde(rename = "userTagIds")]
        pub user_tag_ids: Vec<String>,
    }

    impl From<&DeleteUserTagsIntent> for DeleteUserTagsIntent {
        fn from(value: &DeleteUserTagsIntent) -> Self {
            value.clone()
        }
    }

    ///DeleteUserTagsRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/DeleteUserTagsIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_DELETE_USER_TAGS"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeleteUserTagsRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: DeleteUserTagsIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: DeleteUserTagsRequestType,
    }

    impl From<&DeleteUserTagsRequest> for DeleteUserTagsRequest {
        fn from(value: &DeleteUserTagsRequest) -> Self {
            value.clone()
        }
    }

    ///DeleteUserTagsRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_DELETE_USER_TAGS"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum DeleteUserTagsRequestType {
        #[serde(rename = "ACTIVITY_TYPE_DELETE_USER_TAGS")]
        ActivityTypeDeleteUserTags,
    }

    impl From<&DeleteUserTagsRequestType> for DeleteUserTagsRequestType {
        fn from(value: &DeleteUserTagsRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for DeleteUserTagsRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeDeleteUserTags => write!(f, "ACTIVITY_TYPE_DELETE_USER_TAGS"),
            }
        }
    }

    impl std::str::FromStr for DeleteUserTagsRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_DELETE_USER_TAGS" => Ok(Self::ActivityTypeDeleteUserTags),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for DeleteUserTagsRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for DeleteUserTagsRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for DeleteUserTagsRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///DeleteUserTagsResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "userIds",
    ///    "userTagIds"
    ///  ],
    ///  "properties": {
    ///    "userIds": {
    ///      "description": "A list of User IDs.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "userTagIds": {
    ///      "description": "A list of User Tag IDs.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeleteUserTagsResult {
        ///A list of User IDs.
        #[serde(rename = "userIds")]
        pub user_ids: Vec<String>,
        ///A list of User Tag IDs.
        #[serde(rename = "userTagIds")]
        pub user_tag_ids: Vec<String>,
    }

    impl From<&DeleteUserTagsResult> for DeleteUserTagsResult {
        fn from(value: &DeleteUserTagsResult) -> Self {
            value.clone()
        }
    }

    ///DeleteUsersIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "userIds"
    ///  ],
    ///  "properties": {
    ///    "userIds": {
    ///      "description": "A list of User IDs.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeleteUsersIntent {
        ///A list of User IDs.
        #[serde(rename = "userIds")]
        pub user_ids: Vec<String>,
    }

    impl From<&DeleteUsersIntent> for DeleteUsersIntent {
        fn from(value: &DeleteUsersIntent) -> Self {
            value.clone()
        }
    }

    ///DeleteUsersRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/DeleteUsersIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_DELETE_USERS"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeleteUsersRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: DeleteUsersIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: DeleteUsersRequestType,
    }

    impl From<&DeleteUsersRequest> for DeleteUsersRequest {
        fn from(value: &DeleteUsersRequest) -> Self {
            value.clone()
        }
    }

    ///DeleteUsersRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_DELETE_USERS"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum DeleteUsersRequestType {
        #[serde(rename = "ACTIVITY_TYPE_DELETE_USERS")]
        ActivityTypeDeleteUsers,
    }

    impl From<&DeleteUsersRequestType> for DeleteUsersRequestType {
        fn from(value: &DeleteUsersRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for DeleteUsersRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeDeleteUsers => write!(f, "ACTIVITY_TYPE_DELETE_USERS"),
            }
        }
    }

    impl std::str::FromStr for DeleteUsersRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_DELETE_USERS" => Ok(Self::ActivityTypeDeleteUsers),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for DeleteUsersRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for DeleteUsersRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for DeleteUsersRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///DeleteUsersResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "userIds"
    ///  ],
    ///  "properties": {
    ///    "userIds": {
    ///      "description": "A list of User IDs.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeleteUsersResult {
        ///A list of User IDs.
        #[serde(rename = "userIds")]
        pub user_ids: Vec<String>,
    }

    impl From<&DeleteUsersResult> for DeleteUsersResult {
        fn from(value: &DeleteUsersResult) -> Self {
            value.clone()
        }
    }

    ///DeleteWalletsIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "walletIds"
    ///  ],
    ///  "properties": {
    ///    "deleteWithoutExport": {
    ///      "description": "Optional parameter for deleting the wallets, even
    /// if any have not been previously exported. If they have been exported,
    /// this field is ignored.",
    ///      "type": "boolean"
    ///    },
    ///    "walletIds": {
    ///      "description": "List of unique identifiers for wallets within an
    /// organization",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeleteWalletsIntent {
        ///Optional parameter for deleting the wallets, even if any have not
        /// been previously exported. If they have been exported, this field is
        /// ignored.
        #[serde(
            rename = "deleteWithoutExport",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_without_export: Option<bool>,
        ///List of unique identifiers for wallets within an organization
        #[serde(rename = "walletIds")]
        pub wallet_ids: Vec<String>,
    }

    impl From<&DeleteWalletsIntent> for DeleteWalletsIntent {
        fn from(value: &DeleteWalletsIntent) -> Self {
            value.clone()
        }
    }

    ///DeleteWalletsRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/DeleteWalletsIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_DELETE_WALLETS"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeleteWalletsRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: DeleteWalletsIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: DeleteWalletsRequestType,
    }

    impl From<&DeleteWalletsRequest> for DeleteWalletsRequest {
        fn from(value: &DeleteWalletsRequest) -> Self {
            value.clone()
        }
    }

    ///DeleteWalletsRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_DELETE_WALLETS"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum DeleteWalletsRequestType {
        #[serde(rename = "ACTIVITY_TYPE_DELETE_WALLETS")]
        ActivityTypeDeleteWallets,
    }

    impl From<&DeleteWalletsRequestType> for DeleteWalletsRequestType {
        fn from(value: &DeleteWalletsRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for DeleteWalletsRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeDeleteWallets => write!(f, "ACTIVITY_TYPE_DELETE_WALLETS"),
            }
        }
    }

    impl std::str::FromStr for DeleteWalletsRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_DELETE_WALLETS" => Ok(Self::ActivityTypeDeleteWallets),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for DeleteWalletsRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for DeleteWalletsRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for DeleteWalletsRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///DeleteWalletsResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "walletIds"
    ///  ],
    ///  "properties": {
    ///    "walletIds": {
    ///      "description": "A list of wallet unique identifiers that were
    /// removed",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DeleteWalletsResult {
        ///A list of wallet unique identifiers that were removed
        #[serde(rename = "walletIds")]
        pub wallet_ids: Vec<String>,
    }

    impl From<&DeleteWalletsResult> for DeleteWalletsResult {
        fn from(value: &DeleteWalletsResult) -> Self {
            value.clone()
        }
    }

    ///DisablePrivateKeyIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "privateKeyId"
    ///  ],
    ///  "properties": {
    ///    "privateKeyId": {
    ///      "description": "Unique identifier for a given Private Key.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DisablePrivateKeyIntent {
        ///Unique identifier for a given Private Key.
        #[serde(rename = "privateKeyId")]
        pub private_key_id: String,
    }

    impl From<&DisablePrivateKeyIntent> for DisablePrivateKeyIntent {
        fn from(value: &DisablePrivateKeyIntent) -> Self {
            value.clone()
        }
    }

    ///DisablePrivateKeyResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "privateKeyId"
    ///  ],
    ///  "properties": {
    ///    "privateKeyId": {
    ///      "description": "Unique identifier for a given Private Key.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DisablePrivateKeyResult {
        ///Unique identifier for a given Private Key.
        #[serde(rename = "privateKeyId")]
        pub private_key_id: String,
    }

    impl From<&DisablePrivateKeyResult> for DisablePrivateKeyResult {
        fn from(value: &DisablePrivateKeyResult) -> Self {
            value.clone()
        }
    }

    ///Effect
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "EFFECT_ALLOW",
    ///    "EFFECT_DENY"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum Effect {
        #[serde(rename = "EFFECT_ALLOW")]
        EffectAllow,
        #[serde(rename = "EFFECT_DENY")]
        EffectDeny,
    }

    impl From<&Effect> for Effect {
        fn from(value: &Effect) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for Effect {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::EffectAllow => write!(f, "EFFECT_ALLOW"),
                Self::EffectDeny => write!(f, "EFFECT_DENY"),
            }
        }
    }

    impl std::str::FromStr for Effect {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "EFFECT_ALLOW" => Ok(Self::EffectAllow),
                "EFFECT_DENY" => Ok(Self::EffectDeny),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for Effect {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for Effect {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for Effect {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///EmailAuthIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "email",
    ///    "targetPublicKey"
    ///  ],
    ///  "properties": {
    ///    "apiKeyName": {
    ///      "description": "Optional human-readable name for an API Key. If
    /// none provided, default to Email Auth - <Timestamp>",
    ///      "type": "string"
    ///    },
    ///    "email": {
    ///      "description": "Email of the authenticating user.",
    ///      "type": "string"
    ///    },
    ///    "emailCustomization": {
    ///      "$ref": "#/components/schemas/EmailCustomizationParams"
    ///    },
    ///    "expirationSeconds": {
    ///      "description": "Expiration window (in seconds) indicating how long
    /// the API key is valid. If not provided, a default of 15 minutes will be
    /// used.",
    ///      "type": "string"
    ///    },
    ///    "invalidateExisting": {
    ///      "description": "Invalidate all other previously generated Email
    /// Auth API keys",
    ///      "type": "boolean"
    ///    },
    ///    "targetPublicKey": {
    ///      "description": "Client-side public key generated by the user, to
    /// which the email auth bundle (credentials) will be encrypted.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct EmailAuthIntent {
        ///Optional human-readable name for an API Key. If none provided,
        /// default to Email Auth - <Timestamp>
        #[serde(
            rename = "apiKeyName",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub api_key_name: Option<String>,
        ///Email of the authenticating user.
        pub email: String,
        #[serde(
            rename = "emailCustomization",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub email_customization: Option<EmailCustomizationParams>,
        ///Expiration window (in seconds) indicating how long the API key is
        /// valid. If not provided, a default of 15 minutes will be used.
        #[serde(
            rename = "expirationSeconds",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub expiration_seconds: Option<String>,
        ///Invalidate all other previously generated Email Auth API keys
        #[serde(
            rename = "invalidateExisting",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub invalidate_existing: Option<bool>,
        ///Client-side public key generated by the user, to which the email
        /// auth bundle (credentials) will be encrypted.
        #[serde(rename = "targetPublicKey")]
        pub target_public_key: String,
    }

    impl From<&EmailAuthIntent> for EmailAuthIntent {
        fn from(value: &EmailAuthIntent) -> Self {
            value.clone()
        }
    }

    ///EmailAuthIntentV2
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "email",
    ///    "targetPublicKey"
    ///  ],
    ///  "properties": {
    ///    "apiKeyName": {
    ///      "description": "Optional human-readable name for an API Key. If
    /// none provided, default to Email Auth - <Timestamp>",
    ///      "type": "string"
    ///    },
    ///    "email": {
    ///      "description": "Email of the authenticating user.",
    ///      "type": "string"
    ///    },
    ///    "emailCustomization": {
    ///      "$ref": "#/components/schemas/EmailCustomizationParams"
    ///    },
    ///    "expirationSeconds": {
    ///      "description": "Expiration window (in seconds) indicating how long
    /// the API key is valid. If not provided, a default of 15 minutes will be
    /// used.",
    ///      "type": "string"
    ///    },
    ///    "invalidateExisting": {
    ///      "description": "Invalidate all other previously generated Email
    /// Auth API keys",
    ///      "type": "boolean"
    ///    },
    ///    "targetPublicKey": {
    ///      "description": "Client-side public key generated by the user, to
    /// which the email auth bundle (credentials) will be encrypted.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct EmailAuthIntentV2 {
        ///Optional human-readable name for an API Key. If none provided,
        /// default to Email Auth - <Timestamp>
        #[serde(
            rename = "apiKeyName",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub api_key_name: Option<String>,
        ///Email of the authenticating user.
        pub email: String,
        #[serde(
            rename = "emailCustomization",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub email_customization: Option<EmailCustomizationParams>,
        ///Expiration window (in seconds) indicating how long the API key is
        /// valid. If not provided, a default of 15 minutes will be used.
        #[serde(
            rename = "expirationSeconds",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub expiration_seconds: Option<String>,
        ///Invalidate all other previously generated Email Auth API keys
        #[serde(
            rename = "invalidateExisting",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub invalidate_existing: Option<bool>,
        ///Client-side public key generated by the user, to which the email
        /// auth bundle (credentials) will be encrypted.
        #[serde(rename = "targetPublicKey")]
        pub target_public_key: String,
    }

    impl From<&EmailAuthIntentV2> for EmailAuthIntentV2 {
        fn from(value: &EmailAuthIntentV2) -> Self {
            value.clone()
        }
    }

    ///EmailAuthRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/EmailAuthIntentV2"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_EMAIL_AUTH_V2"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct EmailAuthRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: EmailAuthIntentV2,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: EmailAuthRequestType,
    }

    impl From<&EmailAuthRequest> for EmailAuthRequest {
        fn from(value: &EmailAuthRequest) -> Self {
            value.clone()
        }
    }

    ///EmailAuthRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_EMAIL_AUTH_V2"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum EmailAuthRequestType {
        #[serde(rename = "ACTIVITY_TYPE_EMAIL_AUTH_V2")]
        ActivityTypeEmailAuthV2,
    }

    impl From<&EmailAuthRequestType> for EmailAuthRequestType {
        fn from(value: &EmailAuthRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for EmailAuthRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeEmailAuthV2 => write!(f, "ACTIVITY_TYPE_EMAIL_AUTH_V2"),
            }
        }
    }

    impl std::str::FromStr for EmailAuthRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_EMAIL_AUTH_V2" => Ok(Self::ActivityTypeEmailAuthV2),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for EmailAuthRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for EmailAuthRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for EmailAuthRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///EmailAuthResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "apiKeyId",
    ///    "userId"
    ///  ],
    ///  "properties": {
    ///    "apiKeyId": {
    ///      "description": "Unique identifier for the created API key.",
    ///      "type": "string"
    ///    },
    ///    "userId": {
    ///      "description": "Unique identifier for the authenticating User.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct EmailAuthResult {
        ///Unique identifier for the created API key.
        #[serde(rename = "apiKeyId")]
        pub api_key_id: String,
        ///Unique identifier for the authenticating User.
        #[serde(rename = "userId")]
        pub user_id: String,
    }

    impl From<&EmailAuthResult> for EmailAuthResult {
        fn from(value: &EmailAuthResult) -> Self {
            value.clone()
        }
    }

    ///EmailCustomizationParams
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "appName": {
    ///      "description": "The name of the application.",
    ///      "type": "string"
    ///    },
    ///    "logoUrl": {
    ///      "description": "A URL pointing to a logo in PNG format. Note this
    /// logo will be resized to fit into 340px x 124px.",
    ///      "type": "string"
    ///    },
    ///    "magicLinkTemplate": {
    ///      "description": "A template for the URL to be used in a magic link button, e.g. `https://dapp.xyz/%s`. The auth bundle will be interpolated into the `%s`.",
    ///      "type": "string"
    ///    },
    ///    "templateId": {
    ///      "description": "Unique identifier for a given Email Template. If
    /// not specified, the default is the most recent Email Template.",
    ///      "type": "string"
    ///    },
    ///    "templateVariables": {
    ///      "description": "JSON object containing key/value pairs to be used
    /// with custom templates.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct EmailCustomizationParams {
        ///The name of the application.
        #[serde(rename = "appName", default, skip_serializing_if = "Option::is_none")]
        pub app_name: Option<String>,
        ///A URL pointing to a logo in PNG format. Note this logo will be
        /// resized to fit into 340px x 124px.
        #[serde(rename = "logoUrl", default, skip_serializing_if = "Option::is_none")]
        pub logo_url: Option<String>,
        ///A template for the URL to be used in a magic link button, e.g. `https://dapp.xyz/%s`. The auth bundle will be interpolated into the `%s`.
        #[serde(
            rename = "magicLinkTemplate",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub magic_link_template: Option<String>,
        ///Unique identifier for a given Email Template. If not specified, the
        /// default is the most recent Email Template.
        #[serde(
            rename = "templateId",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub template_id: Option<String>,
        ///JSON object containing key/value pairs to be used with custom
        /// templates.
        #[serde(
            rename = "templateVariables",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub template_variables: Option<String>,
    }

    impl From<&EmailCustomizationParams> for EmailCustomizationParams {
        fn from(value: &EmailCustomizationParams) -> Self {
            value.clone()
        }
    }

    ///ExportPrivateKeyIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "privateKeyId",
    ///    "targetPublicKey"
    ///  ],
    ///  "properties": {
    ///    "privateKeyId": {
    ///      "description": "Unique identifier for a given Private Key.",
    ///      "type": "string"
    ///    },
    ///    "targetPublicKey": {
    ///      "description": "Client-side public key generated by the user, to
    /// which the export bundle will be encrypted.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ExportPrivateKeyIntent {
        ///Unique identifier for a given Private Key.
        #[serde(rename = "privateKeyId")]
        pub private_key_id: String,
        ///Client-side public key generated by the user, to which the export
        /// bundle will be encrypted.
        #[serde(rename = "targetPublicKey")]
        pub target_public_key: String,
    }

    impl From<&ExportPrivateKeyIntent> for ExportPrivateKeyIntent {
        fn from(value: &ExportPrivateKeyIntent) -> Self {
            value.clone()
        }
    }

    ///ExportPrivateKeyRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/ExportPrivateKeyIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_EXPORT_PRIVATE_KEY"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ExportPrivateKeyRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: ExportPrivateKeyIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: ExportPrivateKeyRequestType,
    }

    impl From<&ExportPrivateKeyRequest> for ExportPrivateKeyRequest {
        fn from(value: &ExportPrivateKeyRequest) -> Self {
            value.clone()
        }
    }

    ///ExportPrivateKeyRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_EXPORT_PRIVATE_KEY"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum ExportPrivateKeyRequestType {
        #[serde(rename = "ACTIVITY_TYPE_EXPORT_PRIVATE_KEY")]
        ActivityTypeExportPrivateKey,
    }

    impl From<&ExportPrivateKeyRequestType> for ExportPrivateKeyRequestType {
        fn from(value: &ExportPrivateKeyRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for ExportPrivateKeyRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeExportPrivateKey => write!(f, "ACTIVITY_TYPE_EXPORT_PRIVATE_KEY"),
            }
        }
    }

    impl std::str::FromStr for ExportPrivateKeyRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_EXPORT_PRIVATE_KEY" => Ok(Self::ActivityTypeExportPrivateKey),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for ExportPrivateKeyRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for ExportPrivateKeyRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for ExportPrivateKeyRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///ExportPrivateKeyResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "exportBundle",
    ///    "privateKeyId"
    ///  ],
    ///  "properties": {
    ///    "exportBundle": {
    ///      "description": "Export bundle containing a private key encrypted to
    /// the client's target public key.",
    ///      "type": "string"
    ///    },
    ///    "privateKeyId": {
    ///      "description": "Unique identifier for a given Private Key.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ExportPrivateKeyResult {
        ///Export bundle containing a private key encrypted to the client's
        /// target public key.
        #[serde(rename = "exportBundle")]
        pub export_bundle: String,
        ///Unique identifier for a given Private Key.
        #[serde(rename = "privateKeyId")]
        pub private_key_id: String,
    }

    impl From<&ExportPrivateKeyResult> for ExportPrivateKeyResult {
        fn from(value: &ExportPrivateKeyResult) -> Self {
            value.clone()
        }
    }

    ///ExportWalletAccountIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "address",
    ///    "targetPublicKey"
    ///  ],
    ///  "properties": {
    ///    "address": {
    ///      "description": "Address to identify Wallet Account.",
    ///      "type": "string"
    ///    },
    ///    "targetPublicKey": {
    ///      "description": "Client-side public key generated by the user, to
    /// which the export bundle will be encrypted.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ExportWalletAccountIntent {
        ///Address to identify Wallet Account.
        pub address: String,
        ///Client-side public key generated by the user, to which the export
        /// bundle will be encrypted.
        #[serde(rename = "targetPublicKey")]
        pub target_public_key: String,
    }

    impl From<&ExportWalletAccountIntent> for ExportWalletAccountIntent {
        fn from(value: &ExportWalletAccountIntent) -> Self {
            value.clone()
        }
    }

    ///ExportWalletAccountRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/ExportWalletAccountIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_EXPORT_WALLET_ACCOUNT"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ExportWalletAccountRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: ExportWalletAccountIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: ExportWalletAccountRequestType,
    }

    impl From<&ExportWalletAccountRequest> for ExportWalletAccountRequest {
        fn from(value: &ExportWalletAccountRequest) -> Self {
            value.clone()
        }
    }

    ///ExportWalletAccountRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_EXPORT_WALLET_ACCOUNT"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum ExportWalletAccountRequestType {
        #[serde(rename = "ACTIVITY_TYPE_EXPORT_WALLET_ACCOUNT")]
        ActivityTypeExportWalletAccount,
    }

    impl From<&ExportWalletAccountRequestType> for ExportWalletAccountRequestType {
        fn from(value: &ExportWalletAccountRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for ExportWalletAccountRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeExportWalletAccount => {
                    write!(f, "ACTIVITY_TYPE_EXPORT_WALLET_ACCOUNT")
                }
            }
        }
    }

    impl std::str::FromStr for ExportWalletAccountRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_EXPORT_WALLET_ACCOUNT" => Ok(Self::ActivityTypeExportWalletAccount),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for ExportWalletAccountRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for ExportWalletAccountRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for ExportWalletAccountRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///ExportWalletAccountResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "address",
    ///    "exportBundle"
    ///  ],
    ///  "properties": {
    ///    "address": {
    ///      "description": "Address to identify Wallet Account.",
    ///      "type": "string"
    ///    },
    ///    "exportBundle": {
    ///      "description": "Export bundle containing a private key encrypted by
    /// the client's target public key.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ExportWalletAccountResult {
        ///Address to identify Wallet Account.
        pub address: String,
        ///Export bundle containing a private key encrypted by the client's
        /// target public key.
        #[serde(rename = "exportBundle")]
        pub export_bundle: String,
    }

    impl From<&ExportWalletAccountResult> for ExportWalletAccountResult {
        fn from(value: &ExportWalletAccountResult) -> Self {
            value.clone()
        }
    }

    ///ExportWalletIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "targetPublicKey",
    ///    "walletId"
    ///  ],
    ///  "properties": {
    ///    "language": {
    ///      "$ref": "#/components/schemas/MnemonicLanguage"
    ///    },
    ///    "targetPublicKey": {
    ///      "description": "Client-side public key generated by the user, to
    /// which the export bundle will be encrypted.",
    ///      "type": "string"
    ///    },
    ///    "walletId": {
    ///      "description": "Unique identifier for a given Wallet.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ExportWalletIntent {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub language: Option<MnemonicLanguage>,
        ///Client-side public key generated by the user, to which the export
        /// bundle will be encrypted.
        #[serde(rename = "targetPublicKey")]
        pub target_public_key: String,
        ///Unique identifier for a given Wallet.
        #[serde(rename = "walletId")]
        pub wallet_id: String,
    }

    impl From<&ExportWalletIntent> for ExportWalletIntent {
        fn from(value: &ExportWalletIntent) -> Self {
            value.clone()
        }
    }

    ///ExportWalletRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/ExportWalletIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_EXPORT_WALLET"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ExportWalletRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: ExportWalletIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: ExportWalletRequestType,
    }

    impl From<&ExportWalletRequest> for ExportWalletRequest {
        fn from(value: &ExportWalletRequest) -> Self {
            value.clone()
        }
    }

    ///ExportWalletRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_EXPORT_WALLET"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum ExportWalletRequestType {
        #[serde(rename = "ACTIVITY_TYPE_EXPORT_WALLET")]
        ActivityTypeExportWallet,
    }

    impl From<&ExportWalletRequestType> for ExportWalletRequestType {
        fn from(value: &ExportWalletRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for ExportWalletRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeExportWallet => write!(f, "ACTIVITY_TYPE_EXPORT_WALLET"),
            }
        }
    }

    impl std::str::FromStr for ExportWalletRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_EXPORT_WALLET" => Ok(Self::ActivityTypeExportWallet),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for ExportWalletRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for ExportWalletRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for ExportWalletRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///ExportWalletResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "exportBundle",
    ///    "walletId"
    ///  ],
    ///  "properties": {
    ///    "exportBundle": {
    ///      "description": "Export bundle containing a wallet mnemonic +
    /// optional newline passphrase encrypted by the client's target public
    /// key.",
    ///      "type": "string"
    ///    },
    ///    "walletId": {
    ///      "description": "Unique identifier for a given Wallet.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ExportWalletResult {
        ///Export bundle containing a wallet mnemonic + optional newline
        /// passphrase encrypted by the client's target public key.
        #[serde(rename = "exportBundle")]
        pub export_bundle: String,
        ///Unique identifier for a given Wallet.
        #[serde(rename = "walletId")]
        pub wallet_id: String,
    }

    impl From<&ExportWalletResult> for ExportWalletResult {
        fn from(value: &ExportWalletResult) -> Self {
            value.clone()
        }
    }

    ///ExternalDataV1Credential
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "publicKey",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "publicKey": {
    ///      "description": "The public component of a cryptographic key pair
    /// used to sign messages and transactions.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "$ref": "#/components/schemas/CredentialType"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ExternalDataV1Credential {
        ///The public component of a cryptographic key pair used to sign
        /// messages and transactions.
        #[serde(rename = "publicKey")]
        pub public_key: String,
        #[serde(rename = "type")]
        pub type_: CredentialType,
    }

    impl From<&ExternalDataV1Credential> for ExternalDataV1Credential {
        fn from(value: &ExternalDataV1Credential) -> Self {
            value.clone()
        }
    }

    ///ExternalDataV1Quorum
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "threshold",
    ///    "userIds"
    ///  ],
    ///  "properties": {
    ///    "threshold": {
    ///      "description": "Count of unique approvals required to meet
    /// quorum.",
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "userIds": {
    ///      "description": "Unique identifiers of quorum set members.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ExternalDataV1Quorum {
        ///Count of unique approvals required to meet quorum.
        pub threshold: i32,
        ///Unique identifiers of quorum set members.
        #[serde(rename = "userIds")]
        pub user_ids: Vec<String>,
    }

    impl From<&ExternalDataV1Quorum> for ExternalDataV1Quorum {
        fn from(value: &ExternalDataV1Quorum) -> Self {
            value.clone()
        }
    }

    ///ExternalDataV1Timestamp
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "nanos",
    ///    "seconds"
    ///  ],
    ///  "properties": {
    ///    "nanos": {
    ///      "type": "string"
    ///    },
    ///    "seconds": {
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ExternalDataV1Timestamp {
        pub nanos: String,
        pub seconds: String,
    }

    impl From<&ExternalDataV1Timestamp> for ExternalDataV1Timestamp {
        fn from(value: &ExternalDataV1Timestamp) -> Self {
            value.clone()
        }
    }

    ///Feature
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "name": {
    ///      "$ref": "#/components/schemas/FeatureName"
    ///    },
    ///    "value": {
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct Feature {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub name: Option<FeatureName>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub value: Option<String>,
    }

    impl From<&Feature> for Feature {
        fn from(value: &Feature) -> Self {
            value.clone()
        }
    }

    ///FeatureName
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "FEATURE_NAME_ROOT_USER_EMAIL_RECOVERY",
    ///    "FEATURE_NAME_WEBAUTHN_ORIGINS",
    ///    "FEATURE_NAME_EMAIL_AUTH",
    ///    "FEATURE_NAME_EMAIL_RECOVERY",
    ///    "FEATURE_NAME_WEBHOOK",
    ///    "FEATURE_NAME_SMS_AUTH",
    ///    "FEATURE_NAME_OTP_EMAIL_AUTH"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum FeatureName {
        #[serde(rename = "FEATURE_NAME_ROOT_USER_EMAIL_RECOVERY")]
        FeatureNameRootUserEmailRecovery,
        #[serde(rename = "FEATURE_NAME_WEBAUTHN_ORIGINS")]
        FeatureNameWebauthnOrigins,
        #[serde(rename = "FEATURE_NAME_EMAIL_AUTH")]
        FeatureNameEmailAuth,
        #[serde(rename = "FEATURE_NAME_EMAIL_RECOVERY")]
        FeatureNameEmailRecovery,
        #[serde(rename = "FEATURE_NAME_WEBHOOK")]
        FeatureNameWebhook,
        #[serde(rename = "FEATURE_NAME_SMS_AUTH")]
        FeatureNameSmsAuth,
        #[serde(rename = "FEATURE_NAME_OTP_EMAIL_AUTH")]
        FeatureNameOtpEmailAuth,
    }

    impl From<&FeatureName> for FeatureName {
        fn from(value: &FeatureName) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for FeatureName {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::FeatureNameRootUserEmailRecovery => {
                    write!(f, "FEATURE_NAME_ROOT_USER_EMAIL_RECOVERY")
                }
                Self::FeatureNameWebauthnOrigins => write!(f, "FEATURE_NAME_WEBAUTHN_ORIGINS"),
                Self::FeatureNameEmailAuth => write!(f, "FEATURE_NAME_EMAIL_AUTH"),
                Self::FeatureNameEmailRecovery => write!(f, "FEATURE_NAME_EMAIL_RECOVERY"),
                Self::FeatureNameWebhook => write!(f, "FEATURE_NAME_WEBHOOK"),
                Self::FeatureNameSmsAuth => write!(f, "FEATURE_NAME_SMS_AUTH"),
                Self::FeatureNameOtpEmailAuth => write!(f, "FEATURE_NAME_OTP_EMAIL_AUTH"),
            }
        }
    }

    impl std::str::FromStr for FeatureName {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "FEATURE_NAME_ROOT_USER_EMAIL_RECOVERY" => {
                    Ok(Self::FeatureNameRootUserEmailRecovery)
                }
                "FEATURE_NAME_WEBAUTHN_ORIGINS" => Ok(Self::FeatureNameWebauthnOrigins),
                "FEATURE_NAME_EMAIL_AUTH" => Ok(Self::FeatureNameEmailAuth),
                "FEATURE_NAME_EMAIL_RECOVERY" => Ok(Self::FeatureNameEmailRecovery),
                "FEATURE_NAME_WEBHOOK" => Ok(Self::FeatureNameWebhook),
                "FEATURE_NAME_SMS_AUTH" => Ok(Self::FeatureNameSmsAuth),
                "FEATURE_NAME_OTP_EMAIL_AUTH" => Ok(Self::FeatureNameOtpEmailAuth),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for FeatureName {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for FeatureName {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for FeatureName {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///GetActivitiesRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId"
    ///  ],
    ///  "properties": {
    ///    "filterByStatus": {
    ///      "description": "Array of Activity Statuses filtering which
    /// Activities will be listed in the response.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/ActivityStatus"
    ///      }
    ///    },
    ///    "filterByType": {
    ///      "description": "Array of Activity Types filtering which Activities
    /// will be listed in the response.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/ActivityType"
    ///      }
    ///    },
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "paginationOptions": {
    ///      "$ref": "#/components/schemas/Pagination"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetActivitiesRequest {
        ///Array of Activity Statuses filtering which Activities will be listed
        /// in the response.
        #[serde(
            rename = "filterByStatus",
            default,
            skip_serializing_if = "Vec::is_empty"
        )]
        pub filter_by_status: Vec<ActivityStatus>,
        ///Array of Activity Types filtering which Activities will be listed in
        /// the response.
        #[serde(
            rename = "filterByType",
            default,
            skip_serializing_if = "Vec::is_empty"
        )]
        pub filter_by_type: Vec<ActivityType>,
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        #[serde(
            rename = "paginationOptions",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub pagination_options: Option<Pagination>,
    }

    impl From<&GetActivitiesRequest> for GetActivitiesRequest {
        fn from(value: &GetActivitiesRequest) -> Self {
            value.clone()
        }
    }

    ///GetActivitiesResponse
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "activities"
    ///  ],
    ///  "properties": {
    ///    "activities": {
    ///      "description": "A list of Activities.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/Activity"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetActivitiesResponse {
        ///A list of Activities.
        pub activities: Vec<Activity>,
    }

    impl From<&GetActivitiesResponse> for GetActivitiesResponse {
        fn from(value: &GetActivitiesResponse) -> Self {
            value.clone()
        }
    }

    ///GetActivityRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "activityId",
    ///    "organizationId"
    ///  ],
    ///  "properties": {
    ///    "activityId": {
    ///      "description": "Unique identifier for a given Activity object.",
    ///      "type": "string"
    ///    },
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetActivityRequest {
        ///Unique identifier for a given Activity object.
        #[serde(rename = "activityId")]
        pub activity_id: String,
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
    }

    impl From<&GetActivityRequest> for GetActivityRequest {
        fn from(value: &GetActivityRequest) -> Self {
            value.clone()
        }
    }

    ///GetApiKeyRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "apiKeyId",
    ///    "organizationId"
    ///  ],
    ///  "properties": {
    ///    "apiKeyId": {
    ///      "description": "Unique identifier for a given API key.",
    ///      "type": "string"
    ///    },
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetApiKeyRequest {
        ///Unique identifier for a given API key.
        #[serde(rename = "apiKeyId")]
        pub api_key_id: String,
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
    }

    impl From<&GetApiKeyRequest> for GetApiKeyRequest {
        fn from(value: &GetApiKeyRequest) -> Self {
            value.clone()
        }
    }

    ///GetApiKeyResponse
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "apiKey"
    ///  ],
    ///  "properties": {
    ///    "apiKey": {
    ///      "$ref": "#/components/schemas/ApiKey"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetApiKeyResponse {
        #[serde(rename = "apiKey")]
        pub api_key: ApiKey,
    }

    impl From<&GetApiKeyResponse> for GetApiKeyResponse {
        fn from(value: &GetApiKeyResponse) -> Self {
            value.clone()
        }
    }

    ///GetApiKeysRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "userId": {
    ///      "description": "Unique identifier for a given User.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetApiKeysRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        ///Unique identifier for a given User.
        #[serde(rename = "userId", default, skip_serializing_if = "Option::is_none")]
        pub user_id: Option<String>,
    }

    impl From<&GetApiKeysRequest> for GetApiKeysRequest {
        fn from(value: &GetApiKeysRequest) -> Self {
            value.clone()
        }
    }

    ///GetApiKeysResponse
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "apiKeys"
    ///  ],
    ///  "properties": {
    ///    "apiKeys": {
    ///      "description": "A list of API keys.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/ApiKey"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetApiKeysResponse {
        ///A list of API keys.
        #[serde(rename = "apiKeys")]
        pub api_keys: Vec<ApiKey>,
    }

    impl From<&GetApiKeysResponse> for GetApiKeysResponse {
        fn from(value: &GetApiKeysResponse) -> Self {
            value.clone()
        }
    }

    ///GetAuthenticatorRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "authenticatorId",
    ///    "organizationId"
    ///  ],
    ///  "properties": {
    ///    "authenticatorId": {
    ///      "description": "Unique identifier for a given Authenticator.",
    ///      "type": "string"
    ///    },
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetAuthenticatorRequest {
        ///Unique identifier for a given Authenticator.
        #[serde(rename = "authenticatorId")]
        pub authenticator_id: String,
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
    }

    impl From<&GetAuthenticatorRequest> for GetAuthenticatorRequest {
        fn from(value: &GetAuthenticatorRequest) -> Self {
            value.clone()
        }
    }

    ///GetAuthenticatorResponse
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "authenticator"
    ///  ],
    ///  "properties": {
    ///    "authenticator": {
    ///      "$ref": "#/components/schemas/Authenticator"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetAuthenticatorResponse {
        pub authenticator: Authenticator,
    }

    impl From<&GetAuthenticatorResponse> for GetAuthenticatorResponse {
        fn from(value: &GetAuthenticatorResponse) -> Self {
            value.clone()
        }
    }

    ///GetAuthenticatorsRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "userId"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "userId": {
    ///      "description": "Unique identifier for a given User.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetAuthenticatorsRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        ///Unique identifier for a given User.
        #[serde(rename = "userId")]
        pub user_id: String,
    }

    impl From<&GetAuthenticatorsRequest> for GetAuthenticatorsRequest {
        fn from(value: &GetAuthenticatorsRequest) -> Self {
            value.clone()
        }
    }

    ///GetAuthenticatorsResponse
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "authenticators"
    ///  ],
    ///  "properties": {
    ///    "authenticators": {
    ///      "description": "A list of authenticators.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/Authenticator"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetAuthenticatorsResponse {
        ///A list of authenticators.
        pub authenticators: Vec<Authenticator>,
    }

    impl From<&GetAuthenticatorsResponse> for GetAuthenticatorsResponse {
        fn from(value: &GetAuthenticatorsResponse) -> Self {
            value.clone()
        }
    }

    ///GetOauthProvidersRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "userId": {
    ///      "description": "Unique identifier for a given User.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetOauthProvidersRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        ///Unique identifier for a given User.
        #[serde(rename = "userId", default, skip_serializing_if = "Option::is_none")]
        pub user_id: Option<String>,
    }

    impl From<&GetOauthProvidersRequest> for GetOauthProvidersRequest {
        fn from(value: &GetOauthProvidersRequest) -> Self {
            value.clone()
        }
    }

    ///GetOauthProvidersResponse
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "oauthProviders"
    ///  ],
    ///  "properties": {
    ///    "oauthProviders": {
    ///      "description": "A list of Oauth Providers",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/OauthProvider"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetOauthProvidersResponse {
        ///A list of Oauth Providers
        #[serde(rename = "oauthProviders")]
        pub oauth_providers: Vec<OauthProvider>,
    }

    impl From<&GetOauthProvidersResponse> for GetOauthProvidersResponse {
        fn from(value: &GetOauthProvidersResponse) -> Self {
            value.clone()
        }
    }

    ///GetOrganizationConfigsRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetOrganizationConfigsRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
    }

    impl From<&GetOrganizationConfigsRequest> for GetOrganizationConfigsRequest {
        fn from(value: &GetOrganizationConfigsRequest) -> Self {
            value.clone()
        }
    }

    ///GetOrganizationConfigsResponse
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "configs"
    ///  ],
    ///  "properties": {
    ///    "configs": {
    ///      "$ref": "#/components/schemas/Config"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetOrganizationConfigsResponse {
        pub configs: Config,
    }

    impl From<&GetOrganizationConfigsResponse> for GetOrganizationConfigsResponse {
        fn from(value: &GetOrganizationConfigsResponse) -> Self {
            value.clone()
        }
    }

    ///GetPoliciesRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetPoliciesRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
    }

    impl From<&GetPoliciesRequest> for GetPoliciesRequest {
        fn from(value: &GetPoliciesRequest) -> Self {
            value.clone()
        }
    }

    ///GetPoliciesResponse
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "policies"
    ///  ],
    ///  "properties": {
    ///    "policies": {
    ///      "description": "A list of Policies.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/Policy"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetPoliciesResponse {
        ///A list of Policies.
        pub policies: Vec<Policy>,
    }

    impl From<&GetPoliciesResponse> for GetPoliciesResponse {
        fn from(value: &GetPoliciesResponse) -> Self {
            value.clone()
        }
    }

    ///GetPolicyRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "policyId"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "policyId": {
    ///      "description": "Unique identifier for a given Policy.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetPolicyRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        ///Unique identifier for a given Policy.
        #[serde(rename = "policyId")]
        pub policy_id: String,
    }

    impl From<&GetPolicyRequest> for GetPolicyRequest {
        fn from(value: &GetPolicyRequest) -> Self {
            value.clone()
        }
    }

    ///GetPolicyResponse
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "policy"
    ///  ],
    ///  "properties": {
    ///    "policy": {
    ///      "$ref": "#/components/schemas/Policy"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetPolicyResponse {
        pub policy: Policy,
    }

    impl From<&GetPolicyResponse> for GetPolicyResponse {
        fn from(value: &GetPolicyResponse) -> Self {
            value.clone()
        }
    }

    ///GetPrivateKeyRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "privateKeyId"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "privateKeyId": {
    ///      "description": "Unique identifier for a given Private Key.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetPrivateKeyRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        ///Unique identifier for a given Private Key.
        #[serde(rename = "privateKeyId")]
        pub private_key_id: String,
    }

    impl From<&GetPrivateKeyRequest> for GetPrivateKeyRequest {
        fn from(value: &GetPrivateKeyRequest) -> Self {
            value.clone()
        }
    }

    ///GetPrivateKeyResponse
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "privateKey"
    ///  ],
    ///  "properties": {
    ///    "privateKey": {
    ///      "$ref": "#/components/schemas/PrivateKey"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetPrivateKeyResponse {
        #[serde(rename = "privateKey")]
        pub private_key: PrivateKey,
    }

    impl From<&GetPrivateKeyResponse> for GetPrivateKeyResponse {
        fn from(value: &GetPrivateKeyResponse) -> Self {
            value.clone()
        }
    }

    ///GetPrivateKeysRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetPrivateKeysRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
    }

    impl From<&GetPrivateKeysRequest> for GetPrivateKeysRequest {
        fn from(value: &GetPrivateKeysRequest) -> Self {
            value.clone()
        }
    }

    ///GetPrivateKeysResponse
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "privateKeys"
    ///  ],
    ///  "properties": {
    ///    "privateKeys": {
    ///      "description": "A list of Private Keys.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/PrivateKey"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetPrivateKeysResponse {
        ///A list of Private Keys.
        #[serde(rename = "privateKeys")]
        pub private_keys: Vec<PrivateKey>,
    }

    impl From<&GetPrivateKeysResponse> for GetPrivateKeysResponse {
        fn from(value: &GetPrivateKeysResponse) -> Self {
            value.clone()
        }
    }

    ///GetSubOrgIdsRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId"
    ///  ],
    ///  "properties": {
    ///    "filterType": {
    ///      "description": "Specifies the type of filter to apply, i.e
    /// 'CREDENTIAL_ID', 'NAME', 'USERNAME', 'EMAIL', 'OIDC_TOKEN' or
    /// 'PUBLIC_KEY'",
    ///      "type": "string"
    ///    },
    ///    "filterValue": {
    ///      "description": "The value of the filter to apply for the specified
    /// type. For example, a specific email or name string.",
    ///      "type": "string"
    ///    },
    ///    "organizationId": {
    ///      "description": "Unique identifier for the parent Organization. This
    /// is used to find sub-organizations within it.",
    ///      "type": "string"
    ///    },
    ///    "paginationOptions": {
    ///      "$ref": "#/components/schemas/Pagination"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetSubOrgIdsRequest {
        ///Specifies the type of filter to apply, i.e 'CREDENTIAL_ID', 'NAME',
        /// 'USERNAME', 'EMAIL', 'OIDC_TOKEN' or 'PUBLIC_KEY'
        #[serde(
            rename = "filterType",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub filter_type: Option<String>,
        ///The value of the filter to apply for the specified type. For
        /// example, a specific email or name string.
        #[serde(
            rename = "filterValue",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub filter_value: Option<String>,
        ///Unique identifier for the parent Organization. This is used to find
        /// sub-organizations within it.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        #[serde(
            rename = "paginationOptions",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub pagination_options: Option<Pagination>,
    }

    impl From<&GetSubOrgIdsRequest> for GetSubOrgIdsRequest {
        fn from(value: &GetSubOrgIdsRequest) -> Self {
            value.clone()
        }
    }

    ///GetSubOrgIdsResponse
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationIds"
    ///  ],
    ///  "properties": {
    ///    "organizationIds": {
    ///      "description": "List of unique identifiers for the matching
    /// sub-organizations.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetSubOrgIdsResponse {
        ///List of unique identifiers for the matching sub-organizations.
        #[serde(rename = "organizationIds")]
        pub organization_ids: Vec<String>,
    }

    impl From<&GetSubOrgIdsResponse> for GetSubOrgIdsResponse {
        fn from(value: &GetSubOrgIdsResponse) -> Self {
            value.clone()
        }
    }

    ///GetUserRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "userId"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "userId": {
    ///      "description": "Unique identifier for a given User.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetUserRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        ///Unique identifier for a given User.
        #[serde(rename = "userId")]
        pub user_id: String,
    }

    impl From<&GetUserRequest> for GetUserRequest {
        fn from(value: &GetUserRequest) -> Self {
            value.clone()
        }
    }

    ///GetUserResponse
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "user"
    ///  ],
    ///  "properties": {
    ///    "user": {
    ///      "$ref": "#/components/schemas/User"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetUserResponse {
        pub user: User,
    }

    impl From<&GetUserResponse> for GetUserResponse {
        fn from(value: &GetUserResponse) -> Self {
            value.clone()
        }
    }

    ///GetUsersRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetUsersRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
    }

    impl From<&GetUsersRequest> for GetUsersRequest {
        fn from(value: &GetUsersRequest) -> Self {
            value.clone()
        }
    }

    ///GetUsersResponse
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "users"
    ///  ],
    ///  "properties": {
    ///    "users": {
    ///      "description": "A list of Users.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/User"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetUsersResponse {
        ///A list of Users.
        pub users: Vec<User>,
    }

    impl From<&GetUsersResponse> for GetUsersResponse {
        fn from(value: &GetUsersResponse) -> Self {
            value.clone()
        }
    }

    ///GetWalletAccountsRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "walletId"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "paginationOptions": {
    ///      "$ref": "#/components/schemas/Pagination"
    ///    },
    ///    "walletId": {
    ///      "description": "Unique identifier for a given Wallet.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetWalletAccountsRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        #[serde(
            rename = "paginationOptions",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub pagination_options: Option<Pagination>,
        ///Unique identifier for a given Wallet.
        #[serde(rename = "walletId")]
        pub wallet_id: String,
    }

    impl From<&GetWalletAccountsRequest> for GetWalletAccountsRequest {
        fn from(value: &GetWalletAccountsRequest) -> Self {
            value.clone()
        }
    }

    ///GetWalletAccountsResponse
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "accounts"
    ///  ],
    ///  "properties": {
    ///    "accounts": {
    ///      "description": "A list of Accounts generated from a Wallet that
    /// share a common seed",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/WalletAccount"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetWalletAccountsResponse {
        ///A list of Accounts generated from a Wallet that share a common seed
        pub accounts: Vec<WalletAccount>,
    }

    impl From<&GetWalletAccountsResponse> for GetWalletAccountsResponse {
        fn from(value: &GetWalletAccountsResponse) -> Self {
            value.clone()
        }
    }

    ///GetWalletRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "walletId"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "walletId": {
    ///      "description": "Unique identifier for a given Wallet.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetWalletRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        ///Unique identifier for a given Wallet.
        #[serde(rename = "walletId")]
        pub wallet_id: String,
    }

    impl From<&GetWalletRequest> for GetWalletRequest {
        fn from(value: &GetWalletRequest) -> Self {
            value.clone()
        }
    }

    ///GetWalletResponse
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "wallet"
    ///  ],
    ///  "properties": {
    ///    "wallet": {
    ///      "$ref": "#/components/schemas/Wallet"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetWalletResponse {
        pub wallet: Wallet,
    }

    impl From<&GetWalletResponse> for GetWalletResponse {
        fn from(value: &GetWalletResponse) -> Self {
            value.clone()
        }
    }

    ///GetWalletsRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetWalletsRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
    }

    impl From<&GetWalletsRequest> for GetWalletsRequest {
        fn from(value: &GetWalletsRequest) -> Self {
            value.clone()
        }
    }

    ///GetWalletsResponse
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "wallets"
    ///  ],
    ///  "properties": {
    ///    "wallets": {
    ///      "description": "A list of Wallets.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/Wallet"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetWalletsResponse {
        ///A list of Wallets.
        pub wallets: Vec<Wallet>,
    }

    impl From<&GetWalletsResponse> for GetWalletsResponse {
        fn from(value: &GetWalletsResponse) -> Self {
            value.clone()
        }
    }

    ///GetWhoamiRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization. If the
    /// request is being made by a WebAuthN user and their Sub-Organization ID
    /// is unknown, this can be the Parent Organization ID; using the
    /// Sub-Organization ID when possible is preferred due to performance
    /// reasons.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetWhoamiRequest {
        ///Unique identifier for a given Organization. If the request is being
        /// made by a WebAuthN user and their Sub-Organization ID is unknown,
        /// this can be the Parent Organization ID; using the Sub-Organization
        /// ID when possible is preferred due to performance reasons.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
    }

    impl From<&GetWhoamiRequest> for GetWhoamiRequest {
        fn from(value: &GetWhoamiRequest) -> Self {
            value.clone()
        }
    }

    ///GetWhoamiResponse
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "organizationName",
    ///    "userId",
    ///    "username"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "organizationName": {
    ///      "description": "Human-readable name for an Organization.",
    ///      "type": "string"
    ///    },
    ///    "userId": {
    ///      "description": "Unique identifier for a given User.",
    ///      "type": "string"
    ///    },
    ///    "username": {
    ///      "description": "Human-readable name for a User.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GetWhoamiResponse {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        ///Human-readable name for an Organization.
        #[serde(rename = "organizationName")]
        pub organization_name: String,
        ///Unique identifier for a given User.
        #[serde(rename = "userId")]
        pub user_id: String,
        ///Human-readable name for a User.
        pub username: String,
    }

    impl From<&GetWhoamiResponse> for GetWhoamiResponse {
        fn from(value: &GetWhoamiResponse) -> Self {
            value.clone()
        }
    }

    ///HashFunction
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "HASH_FUNCTION_NO_OP",
    ///    "HASH_FUNCTION_SHA256",
    ///    "HASH_FUNCTION_KECCAK256",
    ///    "HASH_FUNCTION_NOT_APPLICABLE"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum HashFunction {
        #[serde(rename = "HASH_FUNCTION_NO_OP")]
        HashFunctionNoOp,
        #[serde(rename = "HASH_FUNCTION_SHA256")]
        HashFunctionSha256,
        #[serde(rename = "HASH_FUNCTION_KECCAK256")]
        HashFunctionKeccak256,
        #[serde(rename = "HASH_FUNCTION_NOT_APPLICABLE")]
        HashFunctionNotApplicable,
    }

    impl From<&HashFunction> for HashFunction {
        fn from(value: &HashFunction) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for HashFunction {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::HashFunctionNoOp => write!(f, "HASH_FUNCTION_NO_OP"),
                Self::HashFunctionSha256 => write!(f, "HASH_FUNCTION_SHA256"),
                Self::HashFunctionKeccak256 => write!(f, "HASH_FUNCTION_KECCAK256"),
                Self::HashFunctionNotApplicable => write!(f, "HASH_FUNCTION_NOT_APPLICABLE"),
            }
        }
    }

    impl std::str::FromStr for HashFunction {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "HASH_FUNCTION_NO_OP" => Ok(Self::HashFunctionNoOp),
                "HASH_FUNCTION_SHA256" => Ok(Self::HashFunctionSha256),
                "HASH_FUNCTION_KECCAK256" => Ok(Self::HashFunctionKeccak256),
                "HASH_FUNCTION_NOT_APPLICABLE" => Ok(Self::HashFunctionNotApplicable),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for HashFunction {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for HashFunction {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for HashFunction {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///ImportPrivateKeyIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "addressFormats",
    ///    "curve",
    ///    "encryptedBundle",
    ///    "privateKeyName",
    ///    "userId"
    ///  ],
    ///  "properties": {
    ///    "addressFormats": {
    ///      "description": "Cryptocurrency-specific formats for a derived
    /// address (e.g., Ethereum).",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/AddressFormat"
    ///      }
    ///    },
    ///    "curve": {
    ///      "$ref": "#/components/schemas/Curve"
    ///    },
    ///    "encryptedBundle": {
    ///      "description": "Bundle containing a raw private key encrypted to
    /// the enclave's target public key.",
    ///      "type": "string"
    ///    },
    ///    "privateKeyName": {
    ///      "description": "Human-readable name for a Private Key.",
    ///      "type": "string"
    ///    },
    ///    "userId": {
    ///      "description": "The ID of the User importing a Private Key.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ImportPrivateKeyIntent {
        ///Cryptocurrency-specific formats for a derived address (e.g.,
        /// Ethereum).
        #[serde(rename = "addressFormats")]
        pub address_formats: Vec<AddressFormat>,
        pub curve: Curve,
        ///Bundle containing a raw private key encrypted to the enclave's
        /// target public key.
        #[serde(rename = "encryptedBundle")]
        pub encrypted_bundle: String,
        ///Human-readable name for a Private Key.
        #[serde(rename = "privateKeyName")]
        pub private_key_name: String,
        ///The ID of the User importing a Private Key.
        #[serde(rename = "userId")]
        pub user_id: String,
    }

    impl From<&ImportPrivateKeyIntent> for ImportPrivateKeyIntent {
        fn from(value: &ImportPrivateKeyIntent) -> Self {
            value.clone()
        }
    }

    ///ImportPrivateKeyRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/ImportPrivateKeyIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_IMPORT_PRIVATE_KEY"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ImportPrivateKeyRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: ImportPrivateKeyIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: ImportPrivateKeyRequestType,
    }

    impl From<&ImportPrivateKeyRequest> for ImportPrivateKeyRequest {
        fn from(value: &ImportPrivateKeyRequest) -> Self {
            value.clone()
        }
    }

    ///ImportPrivateKeyRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_IMPORT_PRIVATE_KEY"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum ImportPrivateKeyRequestType {
        #[serde(rename = "ACTIVITY_TYPE_IMPORT_PRIVATE_KEY")]
        ActivityTypeImportPrivateKey,
    }

    impl From<&ImportPrivateKeyRequestType> for ImportPrivateKeyRequestType {
        fn from(value: &ImportPrivateKeyRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for ImportPrivateKeyRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeImportPrivateKey => write!(f, "ACTIVITY_TYPE_IMPORT_PRIVATE_KEY"),
            }
        }
    }

    impl std::str::FromStr for ImportPrivateKeyRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_IMPORT_PRIVATE_KEY" => Ok(Self::ActivityTypeImportPrivateKey),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for ImportPrivateKeyRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for ImportPrivateKeyRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for ImportPrivateKeyRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///ImportPrivateKeyResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "addresses",
    ///    "privateKeyId"
    ///  ],
    ///  "properties": {
    ///    "addresses": {
    ///      "description": "A list of addresses.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/activity.v1.Address"
    ///      }
    ///    },
    ///    "privateKeyId": {
    ///      "description": "Unique identifier for a Private Key.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ImportPrivateKeyResult {
        ///A list of addresses.
        pub addresses: Vec<ActivityV1Address>,
        ///Unique identifier for a Private Key.
        #[serde(rename = "privateKeyId")]
        pub private_key_id: String,
    }

    impl From<&ImportPrivateKeyResult> for ImportPrivateKeyResult {
        fn from(value: &ImportPrivateKeyResult) -> Self {
            value.clone()
        }
    }

    ///ImportWalletIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "accounts",
    ///    "encryptedBundle",
    ///    "userId",
    ///    "walletName"
    ///  ],
    ///  "properties": {
    ///    "accounts": {
    ///      "description": "A list of wallet Accounts.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/WalletAccountParams"
    ///      }
    ///    },
    ///    "encryptedBundle": {
    ///      "description": "Bundle containing a wallet mnemonic encrypted to
    /// the enclave's target public key.",
    ///      "type": "string"
    ///    },
    ///    "userId": {
    ///      "description": "The ID of the User importing a Wallet.",
    ///      "type": "string"
    ///    },
    ///    "walletName": {
    ///      "description": "Human-readable name for a Wallet.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ImportWalletIntent {
        ///A list of wallet Accounts.
        pub accounts: Vec<WalletAccountParams>,
        ///Bundle containing a wallet mnemonic encrypted to the enclave's
        /// target public key.
        #[serde(rename = "encryptedBundle")]
        pub encrypted_bundle: String,
        ///The ID of the User importing a Wallet.
        #[serde(rename = "userId")]
        pub user_id: String,
        ///Human-readable name for a Wallet.
        #[serde(rename = "walletName")]
        pub wallet_name: String,
    }

    impl From<&ImportWalletIntent> for ImportWalletIntent {
        fn from(value: &ImportWalletIntent) -> Self {
            value.clone()
        }
    }

    ///ImportWalletRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/ImportWalletIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_IMPORT_WALLET"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ImportWalletRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: ImportWalletIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: ImportWalletRequestType,
    }

    impl From<&ImportWalletRequest> for ImportWalletRequest {
        fn from(value: &ImportWalletRequest) -> Self {
            value.clone()
        }
    }

    ///ImportWalletRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_IMPORT_WALLET"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum ImportWalletRequestType {
        #[serde(rename = "ACTIVITY_TYPE_IMPORT_WALLET")]
        ActivityTypeImportWallet,
    }

    impl From<&ImportWalletRequestType> for ImportWalletRequestType {
        fn from(value: &ImportWalletRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for ImportWalletRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeImportWallet => write!(f, "ACTIVITY_TYPE_IMPORT_WALLET"),
            }
        }
    }

    impl std::str::FromStr for ImportWalletRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_IMPORT_WALLET" => Ok(Self::ActivityTypeImportWallet),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for ImportWalletRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for ImportWalletRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for ImportWalletRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///ImportWalletResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "addresses",
    ///    "walletId"
    ///  ],
    ///  "properties": {
    ///    "addresses": {
    ///      "description": "A list of account addresses.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "walletId": {
    ///      "description": "Unique identifier for a Wallet.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ImportWalletResult {
        ///A list of account addresses.
        pub addresses: Vec<String>,
        ///Unique identifier for a Wallet.
        #[serde(rename = "walletId")]
        pub wallet_id: String,
    }

    impl From<&ImportWalletResult> for ImportWalletResult {
        fn from(value: &ImportWalletResult) -> Self {
            value.clone()
        }
    }

    ///InitImportPrivateKeyIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "userId"
    ///  ],
    ///  "properties": {
    ///    "userId": {
    ///      "description": "The ID of the User importing a Private Key.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct InitImportPrivateKeyIntent {
        ///The ID of the User importing a Private Key.
        #[serde(rename = "userId")]
        pub user_id: String,
    }

    impl From<&InitImportPrivateKeyIntent> for InitImportPrivateKeyIntent {
        fn from(value: &InitImportPrivateKeyIntent) -> Self {
            value.clone()
        }
    }

    ///InitImportPrivateKeyRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/InitImportPrivateKeyIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_INIT_IMPORT_PRIVATE_KEY"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct InitImportPrivateKeyRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: InitImportPrivateKeyIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: InitImportPrivateKeyRequestType,
    }

    impl From<&InitImportPrivateKeyRequest> for InitImportPrivateKeyRequest {
        fn from(value: &InitImportPrivateKeyRequest) -> Self {
            value.clone()
        }
    }

    ///InitImportPrivateKeyRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_INIT_IMPORT_PRIVATE_KEY"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum InitImportPrivateKeyRequestType {
        #[serde(rename = "ACTIVITY_TYPE_INIT_IMPORT_PRIVATE_KEY")]
        ActivityTypeInitImportPrivateKey,
    }

    impl From<&InitImportPrivateKeyRequestType> for InitImportPrivateKeyRequestType {
        fn from(value: &InitImportPrivateKeyRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for InitImportPrivateKeyRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeInitImportPrivateKey => {
                    write!(f, "ACTIVITY_TYPE_INIT_IMPORT_PRIVATE_KEY")
                }
            }
        }
    }

    impl std::str::FromStr for InitImportPrivateKeyRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_INIT_IMPORT_PRIVATE_KEY" => {
                    Ok(Self::ActivityTypeInitImportPrivateKey)
                }
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for InitImportPrivateKeyRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for InitImportPrivateKeyRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for InitImportPrivateKeyRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///InitImportPrivateKeyResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "importBundle"
    ///  ],
    ///  "properties": {
    ///    "importBundle": {
    ///      "description": "Import bundle containing a public key and signature
    /// to use for importing client data.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct InitImportPrivateKeyResult {
        ///Import bundle containing a public key and signature to use for
        /// importing client data.
        #[serde(rename = "importBundle")]
        pub import_bundle: String,
    }

    impl From<&InitImportPrivateKeyResult> for InitImportPrivateKeyResult {
        fn from(value: &InitImportPrivateKeyResult) -> Self {
            value.clone()
        }
    }

    ///InitImportWalletIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "userId"
    ///  ],
    ///  "properties": {
    ///    "userId": {
    ///      "description": "The ID of the User importing a Wallet.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct InitImportWalletIntent {
        ///The ID of the User importing a Wallet.
        #[serde(rename = "userId")]
        pub user_id: String,
    }

    impl From<&InitImportWalletIntent> for InitImportWalletIntent {
        fn from(value: &InitImportWalletIntent) -> Self {
            value.clone()
        }
    }

    ///InitImportWalletRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/InitImportWalletIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_INIT_IMPORT_WALLET"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct InitImportWalletRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: InitImportWalletIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: InitImportWalletRequestType,
    }

    impl From<&InitImportWalletRequest> for InitImportWalletRequest {
        fn from(value: &InitImportWalletRequest) -> Self {
            value.clone()
        }
    }

    ///InitImportWalletRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_INIT_IMPORT_WALLET"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum InitImportWalletRequestType {
        #[serde(rename = "ACTIVITY_TYPE_INIT_IMPORT_WALLET")]
        ActivityTypeInitImportWallet,
    }

    impl From<&InitImportWalletRequestType> for InitImportWalletRequestType {
        fn from(value: &InitImportWalletRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for InitImportWalletRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeInitImportWallet => write!(f, "ACTIVITY_TYPE_INIT_IMPORT_WALLET"),
            }
        }
    }

    impl std::str::FromStr for InitImportWalletRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_INIT_IMPORT_WALLET" => Ok(Self::ActivityTypeInitImportWallet),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for InitImportWalletRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for InitImportWalletRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for InitImportWalletRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///InitImportWalletResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "importBundle"
    ///  ],
    ///  "properties": {
    ///    "importBundle": {
    ///      "description": "Import bundle containing a public key and signature
    /// to use for importing client data.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct InitImportWalletResult {
        ///Import bundle containing a public key and signature to use for
        /// importing client data.
        #[serde(rename = "importBundle")]
        pub import_bundle: String,
    }

    impl From<&InitImportWalletResult> for InitImportWalletResult {
        fn from(value: &InitImportWalletResult) -> Self {
            value.clone()
        }
    }

    ///InitOtpAuthIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "contact",
    ///    "otpType"
    ///  ],
    ///  "properties": {
    ///    "contact": {
    ///      "description": "Email or phone number to send the OTP code to",
    ///      "type": "string"
    ///    },
    ///    "emailCustomization": {
    ///      "$ref": "#/components/schemas/EmailCustomizationParams"
    ///    },
    ///    "otpType": {
    ///      "description": "Enum to specifiy whether to send OTP via SMS or
    /// email",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct InitOtpAuthIntent {
        ///Email or phone number to send the OTP code to
        pub contact: String,
        #[serde(
            rename = "emailCustomization",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub email_customization: Option<EmailCustomizationParams>,
        ///Enum to specifiy whether to send OTP via SMS or email
        #[serde(rename = "otpType")]
        pub otp_type: String,
    }

    impl From<&InitOtpAuthIntent> for InitOtpAuthIntent {
        fn from(value: &InitOtpAuthIntent) -> Self {
            value.clone()
        }
    }

    ///InitOtpAuthRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/InitOtpAuthIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_INIT_OTP_AUTH"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct InitOtpAuthRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: InitOtpAuthIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: InitOtpAuthRequestType,
    }

    impl From<&InitOtpAuthRequest> for InitOtpAuthRequest {
        fn from(value: &InitOtpAuthRequest) -> Self {
            value.clone()
        }
    }

    ///InitOtpAuthRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_INIT_OTP_AUTH"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum InitOtpAuthRequestType {
        #[serde(rename = "ACTIVITY_TYPE_INIT_OTP_AUTH")]
        ActivityTypeInitOtpAuth,
    }

    impl From<&InitOtpAuthRequestType> for InitOtpAuthRequestType {
        fn from(value: &InitOtpAuthRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for InitOtpAuthRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeInitOtpAuth => write!(f, "ACTIVITY_TYPE_INIT_OTP_AUTH"),
            }
        }
    }

    impl std::str::FromStr for InitOtpAuthRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_INIT_OTP_AUTH" => Ok(Self::ActivityTypeInitOtpAuth),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for InitOtpAuthRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for InitOtpAuthRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for InitOtpAuthRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///InitOtpAuthResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "otpId"
    ///  ],
    ///  "properties": {
    ///    "otpId": {
    ///      "description": "Unique identifier for an OTP authentication",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct InitOtpAuthResult {
        ///Unique identifier for an OTP authentication
        #[serde(rename = "otpId")]
        pub otp_id: String,
    }

    impl From<&InitOtpAuthResult> for InitOtpAuthResult {
        fn from(value: &InitOtpAuthResult) -> Self {
            value.clone()
        }
    }

    ///InitUserEmailRecoveryIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "email",
    ///    "targetPublicKey"
    ///  ],
    ///  "properties": {
    ///    "email": {
    ///      "description": "Email of the user starting recovery",
    ///      "type": "string"
    ///    },
    ///    "emailCustomization": {
    ///      "$ref": "#/components/schemas/EmailCustomizationParams"
    ///    },
    ///    "expirationSeconds": {
    ///      "description": "Expiration window (in seconds) indicating how long
    /// the recovery credential is valid. If not provided, a default of 15
    /// minutes will be used.",
    ///      "type": "string"
    ///    },
    ///    "targetPublicKey": {
    ///      "description": "Client-side public key generated by the user, to
    /// which the recovery bundle will be encrypted.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct InitUserEmailRecoveryIntent {
        ///Email of the user starting recovery
        pub email: String,
        #[serde(
            rename = "emailCustomization",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub email_customization: Option<EmailCustomizationParams>,
        ///Expiration window (in seconds) indicating how long the recovery
        /// credential is valid. If not provided, a default of 15 minutes will
        /// be used.
        #[serde(
            rename = "expirationSeconds",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub expiration_seconds: Option<String>,
        ///Client-side public key generated by the user, to which the recovery
        /// bundle will be encrypted.
        #[serde(rename = "targetPublicKey")]
        pub target_public_key: String,
    }

    impl From<&InitUserEmailRecoveryIntent> for InitUserEmailRecoveryIntent {
        fn from(value: &InitUserEmailRecoveryIntent) -> Self {
            value.clone()
        }
    }

    ///InitUserEmailRecoveryRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/InitUserEmailRecoveryIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_INIT_USER_EMAIL_RECOVERY"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct InitUserEmailRecoveryRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: InitUserEmailRecoveryIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: InitUserEmailRecoveryRequestType,
    }

    impl From<&InitUserEmailRecoveryRequest> for InitUserEmailRecoveryRequest {
        fn from(value: &InitUserEmailRecoveryRequest) -> Self {
            value.clone()
        }
    }

    ///InitUserEmailRecoveryRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_INIT_USER_EMAIL_RECOVERY"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum InitUserEmailRecoveryRequestType {
        #[serde(rename = "ACTIVITY_TYPE_INIT_USER_EMAIL_RECOVERY")]
        ActivityTypeInitUserEmailRecovery,
    }

    impl From<&InitUserEmailRecoveryRequestType> for InitUserEmailRecoveryRequestType {
        fn from(value: &InitUserEmailRecoveryRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for InitUserEmailRecoveryRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeInitUserEmailRecovery => {
                    write!(f, "ACTIVITY_TYPE_INIT_USER_EMAIL_RECOVERY")
                }
            }
        }
    }

    impl std::str::FromStr for InitUserEmailRecoveryRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_INIT_USER_EMAIL_RECOVERY" => {
                    Ok(Self::ActivityTypeInitUserEmailRecovery)
                }
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for InitUserEmailRecoveryRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for InitUserEmailRecoveryRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for InitUserEmailRecoveryRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///InitUserEmailRecoveryResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "userId"
    ///  ],
    ///  "properties": {
    ///    "userId": {
    ///      "description": "Unique identifier for the user being recovered.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct InitUserEmailRecoveryResult {
        ///Unique identifier for the user being recovered.
        #[serde(rename = "userId")]
        pub user_id: String,
    }

    impl From<&InitUserEmailRecoveryResult> for InitUserEmailRecoveryResult {
        fn from(value: &InitUserEmailRecoveryResult) -> Self {
            value.clone()
        }
    }

    ///Intent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "acceptInvitationIntent": {
    ///      "$ref": "#/components/schemas/AcceptInvitationIntent"
    ///    },
    ///    "acceptInvitationIntentV2": {
    ///      "$ref": "#/components/schemas/AcceptInvitationIntentV2"
    ///    },
    ///    "activateBillingTierIntent": {
    ///      "$ref": "#/components/schemas/ActivateBillingTierIntent"
    ///    },
    ///    "approveActivityIntent": {
    ///      "$ref": "#/components/schemas/ApproveActivityIntent"
    ///    },
    ///    "createApiKeysIntent": {
    ///      "$ref": "#/components/schemas/CreateApiKeysIntent"
    ///    },
    ///    "createApiKeysIntentV2": {
    ///      "$ref": "#/components/schemas/CreateApiKeysIntentV2"
    ///    },
    ///    "createApiOnlyUsersIntent": {
    ///      "$ref": "#/components/schemas/CreateApiOnlyUsersIntent"
    ///    },
    ///    "createAuthenticatorsIntent": {
    ///      "$ref": "#/components/schemas/CreateAuthenticatorsIntent"
    ///    },
    ///    "createAuthenticatorsIntentV2": {
    ///      "$ref": "#/components/schemas/CreateAuthenticatorsIntentV2"
    ///    },
    ///    "createInvitationsIntent": {
    ///      "$ref": "#/components/schemas/CreateInvitationsIntent"
    ///    },
    ///    "createOauthProvidersIntent": {
    ///      "$ref": "#/components/schemas/CreateOauthProvidersIntent"
    ///    },
    ///    "createOrganizationIntent": {
    ///      "$ref": "#/components/schemas/CreateOrganizationIntent"
    ///    },
    ///    "createOrganizationIntentV2": {
    ///      "$ref": "#/components/schemas/CreateOrganizationIntentV2"
    ///    },
    ///    "createPoliciesIntent": {
    ///      "$ref": "#/components/schemas/CreatePoliciesIntent"
    ///    },
    ///    "createPolicyIntent": {
    ///      "$ref": "#/components/schemas/CreatePolicyIntent"
    ///    },
    ///    "createPolicyIntentV2": {
    ///      "$ref": "#/components/schemas/CreatePolicyIntentV2"
    ///    },
    ///    "createPolicyIntentV3": {
    ///      "$ref": "#/components/schemas/CreatePolicyIntentV3"
    ///    },
    ///    "createPrivateKeyTagIntent": {
    ///      "$ref": "#/components/schemas/CreatePrivateKeyTagIntent"
    ///    },
    ///    "createPrivateKeysIntent": {
    ///      "$ref": "#/components/schemas/CreatePrivateKeysIntent"
    ///    },
    ///    "createPrivateKeysIntentV2": {
    ///      "$ref": "#/components/schemas/CreatePrivateKeysIntentV2"
    ///    },
    ///    "createReadOnlySessionIntent": {
    ///      "$ref": "#/components/schemas/CreateReadOnlySessionIntent"
    ///    },
    ///    "createReadWriteSessionIntent": {
    ///      "$ref": "#/components/schemas/CreateReadWriteSessionIntent"
    ///    },
    ///    "createReadWriteSessionIntentV2": {
    ///      "$ref": "#/components/schemas/CreateReadWriteSessionIntentV2"
    ///    },
    ///    "createSubOrganizationIntent": {
    ///      "$ref": "#/components/schemas/CreateSubOrganizationIntent"
    ///    },
    ///    "createSubOrganizationIntentV2": {
    ///      "$ref": "#/components/schemas/CreateSubOrganizationIntentV2"
    ///    },
    ///    "createSubOrganizationIntentV3": {
    ///      "$ref": "#/components/schemas/CreateSubOrganizationIntentV3"
    ///    },
    ///    "createSubOrganizationIntentV4": {
    ///      "$ref": "#/components/schemas/CreateSubOrganizationIntentV4"
    ///    },
    ///    "createSubOrganizationIntentV5": {
    ///      "$ref": "#/components/schemas/CreateSubOrganizationIntentV5"
    ///    },
    ///    "createSubOrganizationIntentV6": {
    ///      "$ref": "#/components/schemas/CreateSubOrganizationIntentV6"
    ///    },
    ///    "createSubOrganizationIntentV7": {
    ///      "$ref": "#/components/schemas/CreateSubOrganizationIntentV7"
    ///    },
    ///    "createUserTagIntent": {
    ///      "$ref": "#/components/schemas/CreateUserTagIntent"
    ///    },
    ///    "createUsersIntent": {
    ///      "$ref": "#/components/schemas/CreateUsersIntent"
    ///    },
    ///    "createUsersIntentV2": {
    ///      "$ref": "#/components/schemas/CreateUsersIntentV2"
    ///    },
    ///    "createWalletAccountsIntent": {
    ///      "$ref": "#/components/schemas/CreateWalletAccountsIntent"
    ///    },
    ///    "createWalletIntent": {
    ///      "$ref": "#/components/schemas/CreateWalletIntent"
    ///    },
    ///    "deleteApiKeysIntent": {
    ///      "$ref": "#/components/schemas/DeleteApiKeysIntent"
    ///    },
    ///    "deleteAuthenticatorsIntent": {
    ///      "$ref": "#/components/schemas/DeleteAuthenticatorsIntent"
    ///    },
    ///    "deleteInvitationIntent": {
    ///      "$ref": "#/components/schemas/DeleteInvitationIntent"
    ///    },
    ///    "deleteOauthProvidersIntent": {
    ///      "$ref": "#/components/schemas/DeleteOauthProvidersIntent"
    ///    },
    ///    "deleteOrganizationIntent": {
    ///      "$ref": "#/components/schemas/DeleteOrganizationIntent"
    ///    },
    ///    "deletePaymentMethodIntent": {
    ///      "$ref": "#/components/schemas/DeletePaymentMethodIntent"
    ///    },
    ///    "deletePolicyIntent": {
    ///      "$ref": "#/components/schemas/DeletePolicyIntent"
    ///    },
    ///    "deletePrivateKeyTagsIntent": {
    ///      "$ref": "#/components/schemas/DeletePrivateKeyTagsIntent"
    ///    },
    ///    "deletePrivateKeysIntent": {
    ///      "$ref": "#/components/schemas/DeletePrivateKeysIntent"
    ///    },
    ///    "deleteSubOrganizationIntent": {
    ///      "$ref": "#/components/schemas/DeleteSubOrganizationIntent"
    ///    },
    ///    "deleteUserTagsIntent": {
    ///      "$ref": "#/components/schemas/DeleteUserTagsIntent"
    ///    },
    ///    "deleteUsersIntent": {
    ///      "$ref": "#/components/schemas/DeleteUsersIntent"
    ///    },
    ///    "deleteWalletsIntent": {
    ///      "$ref": "#/components/schemas/DeleteWalletsIntent"
    ///    },
    ///    "disablePrivateKeyIntent": {
    ///      "$ref": "#/components/schemas/DisablePrivateKeyIntent"
    ///    },
    ///    "emailAuthIntent": {
    ///      "$ref": "#/components/schemas/EmailAuthIntent"
    ///    },
    ///    "emailAuthIntentV2": {
    ///      "$ref": "#/components/schemas/EmailAuthIntentV2"
    ///    },
    ///    "exportPrivateKeyIntent": {
    ///      "$ref": "#/components/schemas/ExportPrivateKeyIntent"
    ///    },
    ///    "exportWalletAccountIntent": {
    ///      "$ref": "#/components/schemas/ExportWalletAccountIntent"
    ///    },
    ///    "exportWalletIntent": {
    ///      "$ref": "#/components/schemas/ExportWalletIntent"
    ///    },
    ///    "importPrivateKeyIntent": {
    ///      "$ref": "#/components/schemas/ImportPrivateKeyIntent"
    ///    },
    ///    "importWalletIntent": {
    ///      "$ref": "#/components/schemas/ImportWalletIntent"
    ///    },
    ///    "initImportPrivateKeyIntent": {
    ///      "$ref": "#/components/schemas/InitImportPrivateKeyIntent"
    ///    },
    ///    "initImportWalletIntent": {
    ///      "$ref": "#/components/schemas/InitImportWalletIntent"
    ///    },
    ///    "initOtpAuthIntent": {
    ///      "$ref": "#/components/schemas/InitOtpAuthIntent"
    ///    },
    ///    "initUserEmailRecoveryIntent": {
    ///      "$ref": "#/components/schemas/InitUserEmailRecoveryIntent"
    ///    },
    ///    "oauthIntent": {
    ///      "$ref": "#/components/schemas/OauthIntent"
    ///    },
    ///    "otpAuthIntent": {
    ///      "$ref": "#/components/schemas/OtpAuthIntent"
    ///    },
    ///    "recoverUserIntent": {
    ///      "$ref": "#/components/schemas/RecoverUserIntent"
    ///    },
    ///    "rejectActivityIntent": {
    ///      "$ref": "#/components/schemas/RejectActivityIntent"
    ///    },
    ///    "removeOrganizationFeatureIntent": {
    ///      "$ref": "#/components/schemas/RemoveOrganizationFeatureIntent"
    ///    },
    ///    "setOrganizationFeatureIntent": {
    ///      "$ref": "#/components/schemas/SetOrganizationFeatureIntent"
    ///    },
    ///    "setPaymentMethodIntent": {
    ///      "$ref": "#/components/schemas/SetPaymentMethodIntent"
    ///    },
    ///    "setPaymentMethodIntentV2": {
    ///      "$ref": "#/components/schemas/SetPaymentMethodIntentV2"
    ///    },
    ///    "signRawPayloadIntent": {
    ///      "$ref": "#/components/schemas/SignRawPayloadIntent"
    ///    },
    ///    "signRawPayloadIntentV2": {
    ///      "$ref": "#/components/schemas/SignRawPayloadIntentV2"
    ///    },
    ///    "signRawPayloadsIntent": {
    ///      "$ref": "#/components/schemas/SignRawPayloadsIntent"
    ///    },
    ///    "signTransactionIntent": {
    ///      "$ref": "#/components/schemas/SignTransactionIntent"
    ///    },
    ///    "signTransactionIntentV2": {
    ///      "$ref": "#/components/schemas/SignTransactionIntentV2"
    ///    },
    ///    "updateAllowedOriginsIntent": {
    ///      "$ref": "#/components/schemas/UpdateAllowedOriginsIntent"
    ///    },
    ///    "updatePolicyIntent": {
    ///      "$ref": "#/components/schemas/UpdatePolicyIntent"
    ///    },
    ///    "updatePrivateKeyTagIntent": {
    ///      "$ref": "#/components/schemas/UpdatePrivateKeyTagIntent"
    ///    },
    ///    "updateRootQuorumIntent": {
    ///      "$ref": "#/components/schemas/UpdateRootQuorumIntent"
    ///    },
    ///    "updateUserIntent": {
    ///      "$ref": "#/components/schemas/UpdateUserIntent"
    ///    },
    ///    "updateUserTagIntent": {
    ///      "$ref": "#/components/schemas/UpdateUserTagIntent"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct Intent {
        #[serde(
            rename = "acceptInvitationIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub accept_invitation_intent: Option<AcceptInvitationIntent>,
        #[serde(
            rename = "acceptInvitationIntentV2",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub accept_invitation_intent_v2: Option<AcceptInvitationIntentV2>,
        #[serde(
            rename = "activateBillingTierIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub activate_billing_tier_intent: Option<ActivateBillingTierIntent>,
        #[serde(
            rename = "approveActivityIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub approve_activity_intent: Option<ApproveActivityIntent>,
        #[serde(
            rename = "createApiKeysIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_api_keys_intent: Option<CreateApiKeysIntent>,
        #[serde(
            rename = "createApiKeysIntentV2",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_api_keys_intent_v2: Option<CreateApiKeysIntentV2>,
        #[serde(
            rename = "createApiOnlyUsersIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_api_only_users_intent: Option<CreateApiOnlyUsersIntent>,
        #[serde(
            rename = "createAuthenticatorsIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_authenticators_intent: Option<CreateAuthenticatorsIntent>,
        #[serde(
            rename = "createAuthenticatorsIntentV2",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_authenticators_intent_v2: Option<CreateAuthenticatorsIntentV2>,
        #[serde(
            rename = "createInvitationsIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_invitations_intent: Option<CreateInvitationsIntent>,
        #[serde(
            rename = "createOauthProvidersIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_oauth_providers_intent: Option<CreateOauthProvidersIntent>,
        #[serde(
            rename = "createOrganizationIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_organization_intent: Option<CreateOrganizationIntent>,
        #[serde(
            rename = "createOrganizationIntentV2",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_organization_intent_v2: Option<CreateOrganizationIntentV2>,
        #[serde(
            rename = "createPoliciesIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_policies_intent: Option<CreatePoliciesIntent>,
        #[serde(
            rename = "createPolicyIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_policy_intent: Option<CreatePolicyIntent>,
        #[serde(
            rename = "createPolicyIntentV2",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_policy_intent_v2: Option<CreatePolicyIntentV2>,
        #[serde(
            rename = "createPolicyIntentV3",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_policy_intent_v3: Option<CreatePolicyIntentV3>,
        #[serde(
            rename = "createPrivateKeyTagIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_private_key_tag_intent: Option<CreatePrivateKeyTagIntent>,
        #[serde(
            rename = "createPrivateKeysIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_private_keys_intent: Option<CreatePrivateKeysIntent>,
        #[serde(
            rename = "createPrivateKeysIntentV2",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_private_keys_intent_v2: Option<CreatePrivateKeysIntentV2>,
        #[serde(
            rename = "createReadOnlySessionIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_read_only_session_intent: Option<CreateReadOnlySessionIntent>,
        #[serde(
            rename = "createReadWriteSessionIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_read_write_session_intent: Option<CreateReadWriteSessionIntent>,
        #[serde(
            rename = "createReadWriteSessionIntentV2",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_read_write_session_intent_v2: Option<CreateReadWriteSessionIntentV2>,
        #[serde(
            rename = "createSubOrganizationIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_sub_organization_intent: Option<CreateSubOrganizationIntent>,
        #[serde(
            rename = "createSubOrganizationIntentV2",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_sub_organization_intent_v2: Option<CreateSubOrganizationIntentV2>,
        #[serde(
            rename = "createSubOrganizationIntentV3",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_sub_organization_intent_v3: Option<CreateSubOrganizationIntentV3>,
        #[serde(
            rename = "createSubOrganizationIntentV4",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_sub_organization_intent_v4: Option<CreateSubOrganizationIntentV4>,
        #[serde(
            rename = "createSubOrganizationIntentV5",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_sub_organization_intent_v5: Option<CreateSubOrganizationIntentV5>,
        #[serde(
            rename = "createSubOrganizationIntentV6",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_sub_organization_intent_v6: Option<CreateSubOrganizationIntentV6>,
        #[serde(
            rename = "createSubOrganizationIntentV7",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_sub_organization_intent_v7: Option<CreateSubOrganizationIntentV7>,
        #[serde(
            rename = "createUserTagIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_user_tag_intent: Option<CreateUserTagIntent>,
        #[serde(
            rename = "createUsersIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_users_intent: Option<CreateUsersIntent>,
        #[serde(
            rename = "createUsersIntentV2",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_users_intent_v2: Option<CreateUsersIntentV2>,
        #[serde(
            rename = "createWalletAccountsIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_wallet_accounts_intent: Option<CreateWalletAccountsIntent>,
        #[serde(
            rename = "createWalletIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_wallet_intent: Option<CreateWalletIntent>,
        #[serde(
            rename = "deleteApiKeysIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_api_keys_intent: Option<DeleteApiKeysIntent>,
        #[serde(
            rename = "deleteAuthenticatorsIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_authenticators_intent: Option<DeleteAuthenticatorsIntent>,
        #[serde(
            rename = "deleteInvitationIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_invitation_intent: Option<DeleteInvitationIntent>,
        #[serde(
            rename = "deleteOauthProvidersIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_oauth_providers_intent: Option<DeleteOauthProvidersIntent>,
        #[serde(
            rename = "deleteOrganizationIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_organization_intent: Option<DeleteOrganizationIntent>,
        #[serde(
            rename = "deletePaymentMethodIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_payment_method_intent: Option<DeletePaymentMethodIntent>,
        #[serde(
            rename = "deletePolicyIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_policy_intent: Option<DeletePolicyIntent>,
        #[serde(
            rename = "deletePrivateKeyTagsIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_private_key_tags_intent: Option<DeletePrivateKeyTagsIntent>,
        #[serde(
            rename = "deletePrivateKeysIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_private_keys_intent: Option<DeletePrivateKeysIntent>,
        #[serde(
            rename = "deleteSubOrganizationIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_sub_organization_intent: Option<DeleteSubOrganizationIntent>,
        #[serde(
            rename = "deleteUserTagsIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_user_tags_intent: Option<DeleteUserTagsIntent>,
        #[serde(
            rename = "deleteUsersIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_users_intent: Option<DeleteUsersIntent>,
        #[serde(
            rename = "deleteWalletsIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_wallets_intent: Option<DeleteWalletsIntent>,
        #[serde(
            rename = "disablePrivateKeyIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub disable_private_key_intent: Option<DisablePrivateKeyIntent>,
        #[serde(
            rename = "emailAuthIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub email_auth_intent: Option<EmailAuthIntent>,
        #[serde(
            rename = "emailAuthIntentV2",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub email_auth_intent_v2: Option<EmailAuthIntentV2>,
        #[serde(
            rename = "exportPrivateKeyIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub export_private_key_intent: Option<ExportPrivateKeyIntent>,
        #[serde(
            rename = "exportWalletAccountIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub export_wallet_account_intent: Option<ExportWalletAccountIntent>,
        #[serde(
            rename = "exportWalletIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub export_wallet_intent: Option<ExportWalletIntent>,
        #[serde(
            rename = "importPrivateKeyIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub import_private_key_intent: Option<ImportPrivateKeyIntent>,
        #[serde(
            rename = "importWalletIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub import_wallet_intent: Option<ImportWalletIntent>,
        #[serde(
            rename = "initImportPrivateKeyIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub init_import_private_key_intent: Option<InitImportPrivateKeyIntent>,
        #[serde(
            rename = "initImportWalletIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub init_import_wallet_intent: Option<InitImportWalletIntent>,
        #[serde(
            rename = "initOtpAuthIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub init_otp_auth_intent: Option<InitOtpAuthIntent>,
        #[serde(
            rename = "initUserEmailRecoveryIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub init_user_email_recovery_intent: Option<InitUserEmailRecoveryIntent>,
        #[serde(
            rename = "oauthIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub oauth_intent: Option<OauthIntent>,
        #[serde(
            rename = "otpAuthIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub otp_auth_intent: Option<OtpAuthIntent>,
        #[serde(
            rename = "recoverUserIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub recover_user_intent: Option<RecoverUserIntent>,
        #[serde(
            rename = "rejectActivityIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub reject_activity_intent: Option<RejectActivityIntent>,
        #[serde(
            rename = "removeOrganizationFeatureIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub remove_organization_feature_intent: Option<RemoveOrganizationFeatureIntent>,
        #[serde(
            rename = "setOrganizationFeatureIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub set_organization_feature_intent: Option<SetOrganizationFeatureIntent>,
        #[serde(
            rename = "setPaymentMethodIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub set_payment_method_intent: Option<SetPaymentMethodIntent>,
        #[serde(
            rename = "setPaymentMethodIntentV2",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub set_payment_method_intent_v2: Option<SetPaymentMethodIntentV2>,
        #[serde(
            rename = "signRawPayloadIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub sign_raw_payload_intent: Option<SignRawPayloadIntent>,
        #[serde(
            rename = "signRawPayloadIntentV2",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub sign_raw_payload_intent_v2: Option<SignRawPayloadIntentV2>,
        #[serde(
            rename = "signRawPayloadsIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub sign_raw_payloads_intent: Option<SignRawPayloadsIntent>,
        #[serde(
            rename = "signTransactionIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub sign_transaction_intent: Option<SignTransactionIntent>,
        #[serde(
            rename = "signTransactionIntentV2",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub sign_transaction_intent_v2: Option<SignTransactionIntentV2>,
        #[serde(
            rename = "updateAllowedOriginsIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub update_allowed_origins_intent: Option<UpdateAllowedOriginsIntent>,
        #[serde(
            rename = "updatePolicyIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub update_policy_intent: Option<UpdatePolicyIntent>,
        #[serde(
            rename = "updatePrivateKeyTagIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub update_private_key_tag_intent: Option<UpdatePrivateKeyTagIntent>,
        #[serde(
            rename = "updateRootQuorumIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub update_root_quorum_intent: Option<UpdateRootQuorumIntent>,
        #[serde(
            rename = "updateUserIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub update_user_intent: Option<UpdateUserIntent>,
        #[serde(
            rename = "updateUserTagIntent",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub update_user_tag_intent: Option<UpdateUserTagIntent>,
    }

    impl From<&Intent> for Intent {
        fn from(value: &Intent) -> Self {
            value.clone()
        }
    }

    ///InvitationParams
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "accessType",
    ///    "receiverUserEmail",
    ///    "receiverUserName",
    ///    "receiverUserTags",
    ///    "senderUserId"
    ///  ],
    ///  "properties": {
    ///    "accessType": {
    ///      "$ref": "#/components/schemas/AccessType"
    ///    },
    ///    "receiverUserEmail": {
    ///      "description": "The email address of the intended Invitation
    /// recipient.",
    ///      "type": "string"
    ///    },
    ///    "receiverUserName": {
    ///      "description": "The name of the intended Invitation recipient.",
    ///      "type": "string"
    ///    },
    ///    "receiverUserTags": {
    ///      "description": "A list of tags assigned to the Invitation
    /// recipient.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "senderUserId": {
    ///      "description": "Unique identifier for the Sender of an
    /// Invitation.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct InvitationParams {
        #[serde(rename = "accessType")]
        pub access_type: AccessType,
        ///The email address of the intended Invitation recipient.
        #[serde(rename = "receiverUserEmail")]
        pub receiver_user_email: String,
        ///The name of the intended Invitation recipient.
        #[serde(rename = "receiverUserName")]
        pub receiver_user_name: String,
        ///A list of tags assigned to the Invitation recipient.
        #[serde(rename = "receiverUserTags")]
        pub receiver_user_tags: Vec<String>,
        ///Unique identifier for the Sender of an Invitation.
        #[serde(rename = "senderUserId")]
        pub sender_user_id: String,
    }

    impl From<&InvitationParams> for InvitationParams {
        fn from(value: &InvitationParams) -> Self {
            value.clone()
        }
    }

    ///ListPrivateKeyTagsRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ListPrivateKeyTagsRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
    }

    impl From<&ListPrivateKeyTagsRequest> for ListPrivateKeyTagsRequest {
        fn from(value: &ListPrivateKeyTagsRequest) -> Self {
            value.clone()
        }
    }

    ///ListPrivateKeyTagsResponse
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "privateKeyTags"
    ///  ],
    ///  "properties": {
    ///    "privateKeyTags": {
    ///      "description": "A list of Private Key Tags",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/v1.Tag"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ListPrivateKeyTagsResponse {
        ///A list of Private Key Tags
        #[serde(rename = "privateKeyTags")]
        pub private_key_tags: Vec<V1Tag>,
    }

    impl From<&ListPrivateKeyTagsResponse> for ListPrivateKeyTagsResponse {
        fn from(value: &ListPrivateKeyTagsResponse) -> Self {
            value.clone()
        }
    }

    ///ListUserTagsRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ListUserTagsRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
    }

    impl From<&ListUserTagsRequest> for ListUserTagsRequest {
        fn from(value: &ListUserTagsRequest) -> Self {
            value.clone()
        }
    }

    ///ListUserTagsResponse
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "userTags"
    ///  ],
    ///  "properties": {
    ///    "userTags": {
    ///      "description": "A list of User Tags",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/v1.Tag"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct ListUserTagsResponse {
        ///A list of User Tags
        #[serde(rename = "userTags")]
        pub user_tags: Vec<V1Tag>,
    }

    impl From<&ListUserTagsResponse> for ListUserTagsResponse {
        fn from(value: &ListUserTagsResponse) -> Self {
            value.clone()
        }
    }

    ///MnemonicLanguage
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "MNEMONIC_LANGUAGE_ENGLISH",
    ///    "MNEMONIC_LANGUAGE_SIMPLIFIED_CHINESE",
    ///    "MNEMONIC_LANGUAGE_TRADITIONAL_CHINESE",
    ///    "MNEMONIC_LANGUAGE_CZECH",
    ///    "MNEMONIC_LANGUAGE_FRENCH",
    ///    "MNEMONIC_LANGUAGE_ITALIAN",
    ///    "MNEMONIC_LANGUAGE_JAPANESE",
    ///    "MNEMONIC_LANGUAGE_KOREAN",
    ///    "MNEMONIC_LANGUAGE_SPANISH"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum MnemonicLanguage {
        #[serde(rename = "MNEMONIC_LANGUAGE_ENGLISH")]
        MnemonicLanguageEnglish,
        #[serde(rename = "MNEMONIC_LANGUAGE_SIMPLIFIED_CHINESE")]
        MnemonicLanguageSimplifiedChinese,
        #[serde(rename = "MNEMONIC_LANGUAGE_TRADITIONAL_CHINESE")]
        MnemonicLanguageTraditionalChinese,
        #[serde(rename = "MNEMONIC_LANGUAGE_CZECH")]
        MnemonicLanguageCzech,
        #[serde(rename = "MNEMONIC_LANGUAGE_FRENCH")]
        MnemonicLanguageFrench,
        #[serde(rename = "MNEMONIC_LANGUAGE_ITALIAN")]
        MnemonicLanguageItalian,
        #[serde(rename = "MNEMONIC_LANGUAGE_JAPANESE")]
        MnemonicLanguageJapanese,
        #[serde(rename = "MNEMONIC_LANGUAGE_KOREAN")]
        MnemonicLanguageKorean,
        #[serde(rename = "MNEMONIC_LANGUAGE_SPANISH")]
        MnemonicLanguageSpanish,
    }

    impl From<&MnemonicLanguage> for MnemonicLanguage {
        fn from(value: &MnemonicLanguage) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for MnemonicLanguage {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::MnemonicLanguageEnglish => write!(f, "MNEMONIC_LANGUAGE_ENGLISH"),
                Self::MnemonicLanguageSimplifiedChinese => {
                    write!(f, "MNEMONIC_LANGUAGE_SIMPLIFIED_CHINESE")
                }
                Self::MnemonicLanguageTraditionalChinese => {
                    write!(f, "MNEMONIC_LANGUAGE_TRADITIONAL_CHINESE")
                }
                Self::MnemonicLanguageCzech => write!(f, "MNEMONIC_LANGUAGE_CZECH"),
                Self::MnemonicLanguageFrench => write!(f, "MNEMONIC_LANGUAGE_FRENCH"),
                Self::MnemonicLanguageItalian => write!(f, "MNEMONIC_LANGUAGE_ITALIAN"),
                Self::MnemonicLanguageJapanese => write!(f, "MNEMONIC_LANGUAGE_JAPANESE"),
                Self::MnemonicLanguageKorean => write!(f, "MNEMONIC_LANGUAGE_KOREAN"),
                Self::MnemonicLanguageSpanish => write!(f, "MNEMONIC_LANGUAGE_SPANISH"),
            }
        }
    }

    impl std::str::FromStr for MnemonicLanguage {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "MNEMONIC_LANGUAGE_ENGLISH" => Ok(Self::MnemonicLanguageEnglish),
                "MNEMONIC_LANGUAGE_SIMPLIFIED_CHINESE" => {
                    Ok(Self::MnemonicLanguageSimplifiedChinese)
                }
                "MNEMONIC_LANGUAGE_TRADITIONAL_CHINESE" => {
                    Ok(Self::MnemonicLanguageTraditionalChinese)
                }
                "MNEMONIC_LANGUAGE_CZECH" => Ok(Self::MnemonicLanguageCzech),
                "MNEMONIC_LANGUAGE_FRENCH" => Ok(Self::MnemonicLanguageFrench),
                "MNEMONIC_LANGUAGE_ITALIAN" => Ok(Self::MnemonicLanguageItalian),
                "MNEMONIC_LANGUAGE_JAPANESE" => Ok(Self::MnemonicLanguageJapanese),
                "MNEMONIC_LANGUAGE_KOREAN" => Ok(Self::MnemonicLanguageKorean),
                "MNEMONIC_LANGUAGE_SPANISH" => Ok(Self::MnemonicLanguageSpanish),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for MnemonicLanguage {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for MnemonicLanguage {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for MnemonicLanguage {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///OauthIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "oidcToken",
    ///    "targetPublicKey"
    ///  ],
    ///  "properties": {
    ///    "apiKeyName": {
    ///      "description": "Optional human-readable name for an API Key. If
    /// none provided, default to Oauth - <Timestamp>",
    ///      "type": "string"
    ///    },
    ///    "expirationSeconds": {
    ///      "description": "Expiration window (in seconds) indicating how long
    /// the API key is valid. If not provided, a default of 15 minutes will be
    /// used.",
    ///      "type": "string"
    ///    },
    ///    "oidcToken": {
    ///      "description": "Base64 encoded OIDC token",
    ///      "type": "string"
    ///    },
    ///    "targetPublicKey": {
    ///      "description": "Client-side public key generated by the user, to
    /// which the oauth bundle (credentials) will be encrypted.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct OauthIntent {
        ///Optional human-readable name for an API Key. If none provided,
        /// default to Oauth - <Timestamp>
        #[serde(
            rename = "apiKeyName",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub api_key_name: Option<String>,
        ///Expiration window (in seconds) indicating how long the API key is
        /// valid. If not provided, a default of 15 minutes will be used.
        #[serde(
            rename = "expirationSeconds",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub expiration_seconds: Option<String>,
        ///Base64 encoded OIDC token
        #[serde(rename = "oidcToken")]
        pub oidc_token: String,
        ///Client-side public key generated by the user, to which the oauth
        /// bundle (credentials) will be encrypted.
        #[serde(rename = "targetPublicKey")]
        pub target_public_key: String,
    }

    impl From<&OauthIntent> for OauthIntent {
        fn from(value: &OauthIntent) -> Self {
            value.clone()
        }
    }

    ///OauthProvider
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "audience",
    ///    "createdAt",
    ///    "issuer",
    ///    "providerId",
    ///    "providerName",
    ///    "subject",
    ///    "updatedAt"
    ///  ],
    ///  "properties": {
    ///    "audience": {
    ///      "description": "Expected audience ('aud' attribute of the signed
    /// token) which represents the app ID",
    ///      "type": "string"
    ///    },
    ///    "createdAt": {
    ///      "$ref": "#/components/schemas/external.data.v1.Timestamp"
    ///    },
    ///    "issuer": {
    ///      "description": "The issuer of the token, typically a URL indicating the authentication server, e.g https://accounts.google.com",
    ///      "type": "string"
    ///    },
    ///    "providerId": {
    ///      "description": "Unique identifier for an OAuth Provider",
    ///      "type": "string"
    ///    },
    ///    "providerName": {
    ///      "description": "Human-readable name to identify a Provider.",
    ///      "type": "string"
    ///    },
    ///    "subject": {
    ///      "description": "Expected subject ('sub' attribute of the signed
    /// token) which represents the user ID",
    ///      "type": "string"
    ///    },
    ///    "updatedAt": {
    ///      "$ref": "#/components/schemas/external.data.v1.Timestamp"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct OauthProvider {
        ///Expected audience ('aud' attribute of the signed token) which
        /// represents the app ID
        pub audience: String,
        #[serde(rename = "createdAt")]
        pub created_at: ExternalDataV1Timestamp,
        ///The issuer of the token, typically a URL indicating the authentication server, e.g https://accounts.google.com
        pub issuer: String,
        ///Unique identifier for an OAuth Provider
        #[serde(rename = "providerId")]
        pub provider_id: String,
        ///Human-readable name to identify a Provider.
        #[serde(rename = "providerName")]
        pub provider_name: String,
        ///Expected subject ('sub' attribute of the signed token) which
        /// represents the user ID
        pub subject: String,
        #[serde(rename = "updatedAt")]
        pub updated_at: ExternalDataV1Timestamp,
    }

    impl From<&OauthProvider> for OauthProvider {
        fn from(value: &OauthProvider) -> Self {
            value.clone()
        }
    }

    ///OauthProviderParams
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "oidcToken",
    ///    "providerName"
    ///  ],
    ///  "properties": {
    ///    "oidcToken": {
    ///      "description": "Base64 encoded OIDC token",
    ///      "type": "string"
    ///    },
    ///    "providerName": {
    ///      "description": "Human-readable name to identify a Provider.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct OauthProviderParams {
        ///Base64 encoded OIDC token
        #[serde(rename = "oidcToken")]
        pub oidc_token: String,
        ///Human-readable name to identify a Provider.
        #[serde(rename = "providerName")]
        pub provider_name: String,
    }

    impl From<&OauthProviderParams> for OauthProviderParams {
        fn from(value: &OauthProviderParams) -> Self {
            value.clone()
        }
    }

    ///OauthRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/OauthIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_OAUTH"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct OauthRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: OauthIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: OauthRequestType,
    }

    impl From<&OauthRequest> for OauthRequest {
        fn from(value: &OauthRequest) -> Self {
            value.clone()
        }
    }

    ///OauthRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_OAUTH"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum OauthRequestType {
        #[serde(rename = "ACTIVITY_TYPE_OAUTH")]
        ActivityTypeOauth,
    }

    impl From<&OauthRequestType> for OauthRequestType {
        fn from(value: &OauthRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for OauthRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeOauth => write!(f, "ACTIVITY_TYPE_OAUTH"),
            }
        }
    }

    impl std::str::FromStr for OauthRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_OAUTH" => Ok(Self::ActivityTypeOauth),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for OauthRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for OauthRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for OauthRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///OauthResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "apiKeyId",
    ///    "credentialBundle",
    ///    "userId"
    ///  ],
    ///  "properties": {
    ///    "apiKeyId": {
    ///      "description": "Unique identifier for the created API key.",
    ///      "type": "string"
    ///    },
    ///    "credentialBundle": {
    ///      "description": "HPKE encrypted credential bundle",
    ///      "type": "string"
    ///    },
    ///    "userId": {
    ///      "description": "Unique identifier for the authenticating User.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct OauthResult {
        ///Unique identifier for the created API key.
        #[serde(rename = "apiKeyId")]
        pub api_key_id: String,
        ///HPKE encrypted credential bundle
        #[serde(rename = "credentialBundle")]
        pub credential_bundle: String,
        ///Unique identifier for the authenticating User.
        #[serde(rename = "userId")]
        pub user_id: String,
    }

    impl From<&OauthResult> for OauthResult {
        fn from(value: &OauthResult) -> Self {
            value.clone()
        }
    }

    ///Operator
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "OPERATOR_EQUAL",
    ///    "OPERATOR_MORE_THAN",
    ///    "OPERATOR_MORE_THAN_OR_EQUAL",
    ///    "OPERATOR_LESS_THAN",
    ///    "OPERATOR_LESS_THAN_OR_EQUAL",
    ///    "OPERATOR_CONTAINS",
    ///    "OPERATOR_NOT_EQUAL",
    ///    "OPERATOR_IN",
    ///    "OPERATOR_NOT_IN",
    ///    "OPERATOR_CONTAINS_ONE",
    ///    "OPERATOR_CONTAINS_ALL"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum Operator {
        #[serde(rename = "OPERATOR_EQUAL")]
        OperatorEqual,
        #[serde(rename = "OPERATOR_MORE_THAN")]
        OperatorMoreThan,
        #[serde(rename = "OPERATOR_MORE_THAN_OR_EQUAL")]
        OperatorMoreThanOrEqual,
        #[serde(rename = "OPERATOR_LESS_THAN")]
        OperatorLessThan,
        #[serde(rename = "OPERATOR_LESS_THAN_OR_EQUAL")]
        OperatorLessThanOrEqual,
        #[serde(rename = "OPERATOR_CONTAINS")]
        OperatorContains,
        #[serde(rename = "OPERATOR_NOT_EQUAL")]
        OperatorNotEqual,
        #[serde(rename = "OPERATOR_IN")]
        OperatorIn,
        #[serde(rename = "OPERATOR_NOT_IN")]
        OperatorNotIn,
        #[serde(rename = "OPERATOR_CONTAINS_ONE")]
        OperatorContainsOne,
        #[serde(rename = "OPERATOR_CONTAINS_ALL")]
        OperatorContainsAll,
    }

    impl From<&Operator> for Operator {
        fn from(value: &Operator) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for Operator {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::OperatorEqual => write!(f, "OPERATOR_EQUAL"),
                Self::OperatorMoreThan => write!(f, "OPERATOR_MORE_THAN"),
                Self::OperatorMoreThanOrEqual => write!(f, "OPERATOR_MORE_THAN_OR_EQUAL"),
                Self::OperatorLessThan => write!(f, "OPERATOR_LESS_THAN"),
                Self::OperatorLessThanOrEqual => write!(f, "OPERATOR_LESS_THAN_OR_EQUAL"),
                Self::OperatorContains => write!(f, "OPERATOR_CONTAINS"),
                Self::OperatorNotEqual => write!(f, "OPERATOR_NOT_EQUAL"),
                Self::OperatorIn => write!(f, "OPERATOR_IN"),
                Self::OperatorNotIn => write!(f, "OPERATOR_NOT_IN"),
                Self::OperatorContainsOne => write!(f, "OPERATOR_CONTAINS_ONE"),
                Self::OperatorContainsAll => write!(f, "OPERATOR_CONTAINS_ALL"),
            }
        }
    }

    impl std::str::FromStr for Operator {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "OPERATOR_EQUAL" => Ok(Self::OperatorEqual),
                "OPERATOR_MORE_THAN" => Ok(Self::OperatorMoreThan),
                "OPERATOR_MORE_THAN_OR_EQUAL" => Ok(Self::OperatorMoreThanOrEqual),
                "OPERATOR_LESS_THAN" => Ok(Self::OperatorLessThan),
                "OPERATOR_LESS_THAN_OR_EQUAL" => Ok(Self::OperatorLessThanOrEqual),
                "OPERATOR_CONTAINS" => Ok(Self::OperatorContains),
                "OPERATOR_NOT_EQUAL" => Ok(Self::OperatorNotEqual),
                "OPERATOR_IN" => Ok(Self::OperatorIn),
                "OPERATOR_NOT_IN" => Ok(Self::OperatorNotIn),
                "OPERATOR_CONTAINS_ONE" => Ok(Self::OperatorContainsOne),
                "OPERATOR_CONTAINS_ALL" => Ok(Self::OperatorContainsAll),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for Operator {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for Operator {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for Operator {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///OtpAuthIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "otpCode",
    ///    "otpId"
    ///  ],
    ///  "properties": {
    ///    "apiKeyName": {
    ///      "description": "Optional human-readable name for an API Key. If
    /// none provided, default to OTP Auth - <Timestamp>",
    ///      "type": "string"
    ///    },
    ///    "expirationSeconds": {
    ///      "description": "Expiration window (in seconds) indicating how long
    /// the API key is valid. If not provided, a default of 15 minutes will be
    /// used.",
    ///      "type": "string"
    ///    },
    ///    "invalidateExisting": {
    ///      "description": "Invalidate all other previously generated OTP Auth
    /// API keys",
    ///      "type": "boolean"
    ///    },
    ///    "otpCode": {
    ///      "description": "6 digit OTP code sent out to a user's contact
    /// (email or SMS)",
    ///      "type": "string"
    ///    },
    ///    "otpId": {
    ///      "description": "ID representing the result of an init OTP
    /// activity.",
    ///      "type": "string"
    ///    },
    ///    "targetPublicKey": {
    ///      "description": "Client-side public key generated by the user, to
    /// which the OTP bundle (credentials) will be encrypted.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct OtpAuthIntent {
        ///Optional human-readable name for an API Key. If none provided,
        /// default to OTP Auth - <Timestamp>
        #[serde(
            rename = "apiKeyName",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub api_key_name: Option<String>,
        ///Expiration window (in seconds) indicating how long the API key is
        /// valid. If not provided, a default of 15 minutes will be used.
        #[serde(
            rename = "expirationSeconds",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub expiration_seconds: Option<String>,
        ///Invalidate all other previously generated OTP Auth API keys
        #[serde(
            rename = "invalidateExisting",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub invalidate_existing: Option<bool>,
        ///6 digit OTP code sent out to a user's contact (email or SMS)
        #[serde(rename = "otpCode")]
        pub otp_code: String,
        ///ID representing the result of an init OTP activity.
        #[serde(rename = "otpId")]
        pub otp_id: String,
        ///Client-side public key generated by the user, to which the OTP
        /// bundle (credentials) will be encrypted.
        #[serde(
            rename = "targetPublicKey",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub target_public_key: Option<String>,
    }

    impl From<&OtpAuthIntent> for OtpAuthIntent {
        fn from(value: &OtpAuthIntent) -> Self {
            value.clone()
        }
    }

    ///OtpAuthRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/OtpAuthIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_OTP_AUTH"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct OtpAuthRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: OtpAuthIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: OtpAuthRequestType,
    }

    impl From<&OtpAuthRequest> for OtpAuthRequest {
        fn from(value: &OtpAuthRequest) -> Self {
            value.clone()
        }
    }

    ///OtpAuthRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_OTP_AUTH"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum OtpAuthRequestType {
        #[serde(rename = "ACTIVITY_TYPE_OTP_AUTH")]
        ActivityTypeOtpAuth,
    }

    impl From<&OtpAuthRequestType> for OtpAuthRequestType {
        fn from(value: &OtpAuthRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for OtpAuthRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeOtpAuth => write!(f, "ACTIVITY_TYPE_OTP_AUTH"),
            }
        }
    }

    impl std::str::FromStr for OtpAuthRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_OTP_AUTH" => Ok(Self::ActivityTypeOtpAuth),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for OtpAuthRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for OtpAuthRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for OtpAuthRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///OtpAuthResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "userId"
    ///  ],
    ///  "properties": {
    ///    "apiKeyId": {
    ///      "description": "Unique identifier for the created API key.",
    ///      "type": "string"
    ///    },
    ///    "credentialBundle": {
    ///      "description": "HPKE encrypted credential bundle",
    ///      "type": "string"
    ///    },
    ///    "userId": {
    ///      "description": "Unique identifier for the authenticating User.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct OtpAuthResult {
        ///Unique identifier for the created API key.
        #[serde(rename = "apiKeyId", default, skip_serializing_if = "Option::is_none")]
        pub api_key_id: Option<String>,
        ///HPKE encrypted credential bundle
        #[serde(
            rename = "credentialBundle",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub credential_bundle: Option<String>,
        ///Unique identifier for the authenticating User.
        #[serde(rename = "userId")]
        pub user_id: String,
    }

    impl From<&OtpAuthResult> for OtpAuthResult {
        fn from(value: &OtpAuthResult) -> Self {
            value.clone()
        }
    }

    ///Pagination
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "after": {
    ///      "description": "A pagination cursor. This is an object ID that
    /// enables you to fetch all objects after this ID.",
    ///      "type": "string"
    ///    },
    ///    "before": {
    ///      "description": "A pagination cursor. This is an object ID that
    /// enables you to fetch all objects before this ID.",
    ///      "type": "string"
    ///    },
    ///    "limit": {
    ///      "description": "A limit of the number of object to be returned,
    /// between 1 and 100. Defaults to 10.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct Pagination {
        ///A pagination cursor. This is an object ID that enables you to fetch
        /// all objects after this ID.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub after: Option<String>,
        ///A pagination cursor. This is an object ID that enables you to fetch
        /// all objects before this ID.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub before: Option<String>,
        ///A limit of the number of object to be returned, between 1 and 100.
        /// Defaults to 10.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub limit: Option<String>,
    }

    impl From<&Pagination> for Pagination {
        fn from(value: &Pagination) -> Self {
            value.clone()
        }
    }

    ///PathFormat
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "PATH_FORMAT_BIP32"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum PathFormat {
        #[serde(rename = "PATH_FORMAT_BIP32")]
        PathFormatBip32,
    }

    impl From<&PathFormat> for PathFormat {
        fn from(value: &PathFormat) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for PathFormat {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::PathFormatBip32 => write!(f, "PATH_FORMAT_BIP32"),
            }
        }
    }

    impl std::str::FromStr for PathFormat {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "PATH_FORMAT_BIP32" => Ok(Self::PathFormatBip32),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for PathFormat {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for PathFormat {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for PathFormat {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///PayloadEncoding
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "PAYLOAD_ENCODING_HEXADECIMAL",
    ///    "PAYLOAD_ENCODING_TEXT_UTF8"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum PayloadEncoding {
        #[serde(rename = "PAYLOAD_ENCODING_HEXADECIMAL")]
        PayloadEncodingHexadecimal,
        #[serde(rename = "PAYLOAD_ENCODING_TEXT_UTF8")]
        PayloadEncodingTextUtf8,
    }

    impl From<&PayloadEncoding> for PayloadEncoding {
        fn from(value: &PayloadEncoding) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for PayloadEncoding {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::PayloadEncodingHexadecimal => write!(f, "PAYLOAD_ENCODING_HEXADECIMAL"),
                Self::PayloadEncodingTextUtf8 => write!(f, "PAYLOAD_ENCODING_TEXT_UTF8"),
            }
        }
    }

    impl std::str::FromStr for PayloadEncoding {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "PAYLOAD_ENCODING_HEXADECIMAL" => Ok(Self::PayloadEncodingHexadecimal),
                "PAYLOAD_ENCODING_TEXT_UTF8" => Ok(Self::PayloadEncodingTextUtf8),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for PayloadEncoding {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for PayloadEncoding {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for PayloadEncoding {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///Policy
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "condition",
    ///    "consensus",
    ///    "createdAt",
    ///    "effect",
    ///    "notes",
    ///    "policyId",
    ///    "policyName",
    ///    "updatedAt"
    ///  ],
    ///  "properties": {
    ///    "condition": {
    ///      "description": "A condition expression that evalutes to true or
    /// false.",
    ///      "type": "string"
    ///    },
    ///    "consensus": {
    ///      "description": "A consensus expression that evalutes to true or
    /// false.",
    ///      "type": "string"
    ///    },
    ///    "createdAt": {
    ///      "$ref": "#/components/schemas/external.data.v1.Timestamp"
    ///    },
    ///    "effect": {
    ///      "$ref": "#/components/schemas/Effect"
    ///    },
    ///    "notes": {
    ///      "description": "Human-readable notes added by a User to describe a
    /// particular policy.",
    ///      "type": "string"
    ///    },
    ///    "policyId": {
    ///      "description": "Unique identifier for a given Policy.",
    ///      "type": "string"
    ///    },
    ///    "policyName": {
    ///      "description": "Human-readable name for a Policy.",
    ///      "type": "string"
    ///    },
    ///    "updatedAt": {
    ///      "$ref": "#/components/schemas/external.data.v1.Timestamp"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct Policy {
        ///A condition expression that evalutes to true or false.
        pub condition: String,
        ///A consensus expression that evalutes to true or false.
        pub consensus: String,
        #[serde(rename = "createdAt")]
        pub created_at: ExternalDataV1Timestamp,
        pub effect: Effect,
        ///Human-readable notes added by a User to describe a particular
        /// policy.
        pub notes: String,
        ///Unique identifier for a given Policy.
        #[serde(rename = "policyId")]
        pub policy_id: String,
        ///Human-readable name for a Policy.
        #[serde(rename = "policyName")]
        pub policy_name: String,
        #[serde(rename = "updatedAt")]
        pub updated_at: ExternalDataV1Timestamp,
    }

    impl From<&Policy> for Policy {
        fn from(value: &Policy) -> Self {
            value.clone()
        }
    }

    ///PrivateKey
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "addresses",
    ///    "createdAt",
    ///    "curve",
    ///    "exported",
    ///    "imported",
    ///    "privateKeyId",
    ///    "privateKeyName",
    ///    "privateKeyTags",
    ///    "publicKey",
    ///    "updatedAt"
    ///  ],
    ///  "properties": {
    ///    "addresses": {
    ///      "description": "Derived cryptocurrency addresses for a given
    /// Private Key.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/data.v1.Address"
    ///      }
    ///    },
    ///    "createdAt": {
    ///      "$ref": "#/components/schemas/external.data.v1.Timestamp"
    ///    },
    ///    "curve": {
    ///      "$ref": "#/components/schemas/Curve"
    ///    },
    ///    "exported": {
    ///      "description": "True when a given Private Key is exported, false
    /// otherwise.",
    ///      "type": "boolean"
    ///    },
    ///    "imported": {
    ///      "description": "True when a given Private Key is imported, false
    /// otherwise.",
    ///      "type": "boolean"
    ///    },
    ///    "privateKeyId": {
    ///      "description": "Unique identifier for a given Private Key.",
    ///      "type": "string"
    ///    },
    ///    "privateKeyName": {
    ///      "description": "Human-readable name for a Private Key.",
    ///      "type": "string"
    ///    },
    ///    "privateKeyTags": {
    ///      "description": "A list of Private Key Tag IDs.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "publicKey": {
    ///      "description": "The public component of a cryptographic key pair
    /// used to sign messages and transactions.",
    ///      "type": "string"
    ///    },
    ///    "updatedAt": {
    ///      "$ref": "#/components/schemas/external.data.v1.Timestamp"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct PrivateKey {
        ///Derived cryptocurrency addresses for a given Private Key.
        pub addresses: Vec<DataV1Address>,
        #[serde(rename = "createdAt")]
        pub created_at: ExternalDataV1Timestamp,
        pub curve: Curve,
        ///True when a given Private Key is exported, false otherwise.
        pub exported: bool,
        ///True when a given Private Key is imported, false otherwise.
        pub imported: bool,
        ///Unique identifier for a given Private Key.
        #[serde(rename = "privateKeyId")]
        pub private_key_id: String,
        ///Human-readable name for a Private Key.
        #[serde(rename = "privateKeyName")]
        pub private_key_name: String,
        ///A list of Private Key Tag IDs.
        #[serde(rename = "privateKeyTags")]
        pub private_key_tags: Vec<String>,
        ///The public component of a cryptographic key pair used to sign
        /// messages and transactions.
        #[serde(rename = "publicKey")]
        pub public_key: String,
        #[serde(rename = "updatedAt")]
        pub updated_at: ExternalDataV1Timestamp,
    }

    impl From<&PrivateKey> for PrivateKey {
        fn from(value: &PrivateKey) -> Self {
            value.clone()
        }
    }

    ///PrivateKeyParams
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "addressFormats",
    ///    "curve",
    ///    "privateKeyName",
    ///    "privateKeyTags"
    ///  ],
    ///  "properties": {
    ///    "addressFormats": {
    ///      "description": "Cryptocurrency-specific formats for a derived
    /// address (e.g., Ethereum).",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/AddressFormat"
    ///      }
    ///    },
    ///    "curve": {
    ///      "$ref": "#/components/schemas/Curve"
    ///    },
    ///    "privateKeyName": {
    ///      "description": "Human-readable name for a Private Key.",
    ///      "type": "string"
    ///    },
    ///    "privateKeyTags": {
    ///      "description": "A list of Private Key Tag IDs.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct PrivateKeyParams {
        ///Cryptocurrency-specific formats for a derived address (e.g.,
        /// Ethereum).
        #[serde(rename = "addressFormats")]
        pub address_formats: Vec<AddressFormat>,
        pub curve: Curve,
        ///Human-readable name for a Private Key.
        #[serde(rename = "privateKeyName")]
        pub private_key_name: String,
        ///A list of Private Key Tag IDs.
        #[serde(rename = "privateKeyTags")]
        pub private_key_tags: Vec<String>,
    }

    impl From<&PrivateKeyParams> for PrivateKeyParams {
        fn from(value: &PrivateKeyParams) -> Self {
            value.clone()
        }
    }

    ///PrivateKeyResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "addresses": {
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/activity.v1.Address"
    ///      }
    ///    },
    ///    "privateKeyId": {
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct PrivateKeyResult {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub addresses: Vec<ActivityV1Address>,
        #[serde(
            rename = "privateKeyId",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub private_key_id: Option<String>,
    }

    impl From<&PrivateKeyResult> for PrivateKeyResult {
        fn from(value: &PrivateKeyResult) -> Self {
            value.clone()
        }
    }

    ///PublicKeyCredentialWithAttestation
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "clientExtensionResults",
    ///    "id",
    ///    "rawId",
    ///    "response",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "authenticatorAttachment": {
    ///      "type": [
    ///        "string",
    ///        "null"
    ///      ],
    ///      "enum": [
    ///        "cross-platform",
    ///        "platform"
    ///      ]
    ///    },
    ///    "clientExtensionResults": {
    ///      "$ref": "#/components/schemas/SimpleClientExtensionResults"
    ///    },
    ///    "id": {
    ///      "type": "string"
    ///    },
    ///    "rawId": {
    ///      "type": "string"
    ///    },
    ///    "response": {
    ///      "$ref": "#/components/schemas/AuthenticatorAttestationResponse"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "public-key"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct PublicKeyCredentialWithAttestation {
        #[serde(
            rename = "authenticatorAttachment",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub authenticator_attachment:
            Option<PublicKeyCredentialWithAttestationAuthenticatorAttachment>,
        #[serde(rename = "clientExtensionResults")]
        pub client_extension_results: SimpleClientExtensionResults,
        pub id: String,
        #[serde(rename = "rawId")]
        pub raw_id: String,
        pub response: AuthenticatorAttestationResponse,
        #[serde(rename = "type")]
        pub type_: PublicKeyCredentialWithAttestationType,
    }

    impl From<&PublicKeyCredentialWithAttestation> for PublicKeyCredentialWithAttestation {
        fn from(value: &PublicKeyCredentialWithAttestation) -> Self {
            value.clone()
        }
    }

    ///PublicKeyCredentialWithAttestationAuthenticatorAttachment
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "cross-platform",
    ///    "platform"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum PublicKeyCredentialWithAttestationAuthenticatorAttachment {
        #[serde(rename = "cross-platform")]
        CrossPlatform,
        #[serde(rename = "platform")]
        Platform,
    }

    impl From<&PublicKeyCredentialWithAttestationAuthenticatorAttachment>
        for PublicKeyCredentialWithAttestationAuthenticatorAttachment
    {
        fn from(value: &PublicKeyCredentialWithAttestationAuthenticatorAttachment) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for PublicKeyCredentialWithAttestationAuthenticatorAttachment {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::CrossPlatform => write!(f, "cross-platform"),
                Self::Platform => write!(f, "platform"),
            }
        }
    }

    impl std::str::FromStr for PublicKeyCredentialWithAttestationAuthenticatorAttachment {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "cross-platform" => Ok(Self::CrossPlatform),
                "platform" => Ok(Self::Platform),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for PublicKeyCredentialWithAttestationAuthenticatorAttachment {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for PublicKeyCredentialWithAttestationAuthenticatorAttachment {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for PublicKeyCredentialWithAttestationAuthenticatorAttachment {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///PublicKeyCredentialWithAttestationType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "public-key"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum PublicKeyCredentialWithAttestationType {
        #[serde(rename = "public-key")]
        PublicKey,
    }

    impl From<&PublicKeyCredentialWithAttestationType> for PublicKeyCredentialWithAttestationType {
        fn from(value: &PublicKeyCredentialWithAttestationType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for PublicKeyCredentialWithAttestationType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::PublicKey => write!(f, "public-key"),
            }
        }
    }

    impl std::str::FromStr for PublicKeyCredentialWithAttestationType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "public-key" => Ok(Self::PublicKey),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for PublicKeyCredentialWithAttestationType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for PublicKeyCredentialWithAttestationType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for PublicKeyCredentialWithAttestationType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///RecoverUserIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "authenticator",
    ///    "userId"
    ///  ],
    ///  "properties": {
    ///    "authenticator": {
    ///      "$ref": "#/components/schemas/AuthenticatorParamsV2"
    ///    },
    ///    "userId": {
    ///      "description": "Unique identifier for the user performing
    /// recovery.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct RecoverUserIntent {
        pub authenticator: AuthenticatorParamsV2,
        ///Unique identifier for the user performing recovery.
        #[serde(rename = "userId")]
        pub user_id: String,
    }

    impl From<&RecoverUserIntent> for RecoverUserIntent {
        fn from(value: &RecoverUserIntent) -> Self {
            value.clone()
        }
    }

    ///RecoverUserRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/RecoverUserIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_RECOVER_USER"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct RecoverUserRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: RecoverUserIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: RecoverUserRequestType,
    }

    impl From<&RecoverUserRequest> for RecoverUserRequest {
        fn from(value: &RecoverUserRequest) -> Self {
            value.clone()
        }
    }

    ///RecoverUserRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_RECOVER_USER"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum RecoverUserRequestType {
        #[serde(rename = "ACTIVITY_TYPE_RECOVER_USER")]
        ActivityTypeRecoverUser,
    }

    impl From<&RecoverUserRequestType> for RecoverUserRequestType {
        fn from(value: &RecoverUserRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for RecoverUserRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeRecoverUser => write!(f, "ACTIVITY_TYPE_RECOVER_USER"),
            }
        }
    }

    impl std::str::FromStr for RecoverUserRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_RECOVER_USER" => Ok(Self::ActivityTypeRecoverUser),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for RecoverUserRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for RecoverUserRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for RecoverUserRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///RecoverUserResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "authenticatorId"
    ///  ],
    ///  "properties": {
    ///    "authenticatorId": {
    ///      "description": "ID of the authenticator created.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct RecoverUserResult {
        ///ID of the authenticator created.
        #[serde(rename = "authenticatorId")]
        pub authenticator_id: Vec<String>,
    }

    impl From<&RecoverUserResult> for RecoverUserResult {
        fn from(value: &RecoverUserResult) -> Self {
            value.clone()
        }
    }

    ///RejectActivityIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "fingerprint"
    ///  ],
    ///  "properties": {
    ///    "fingerprint": {
    ///      "description": "An artifact verifying a User's action.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct RejectActivityIntent {
        ///An artifact verifying a User's action.
        pub fingerprint: String,
    }

    impl From<&RejectActivityIntent> for RejectActivityIntent {
        fn from(value: &RejectActivityIntent) -> Self {
            value.clone()
        }
    }

    ///RejectActivityRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/RejectActivityIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_REJECT_ACTIVITY"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct RejectActivityRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: RejectActivityIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: RejectActivityRequestType,
    }

    impl From<&RejectActivityRequest> for RejectActivityRequest {
        fn from(value: &RejectActivityRequest) -> Self {
            value.clone()
        }
    }

    ///RejectActivityRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_REJECT_ACTIVITY"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum RejectActivityRequestType {
        #[serde(rename = "ACTIVITY_TYPE_REJECT_ACTIVITY")]
        ActivityTypeRejectActivity,
    }

    impl From<&RejectActivityRequestType> for RejectActivityRequestType {
        fn from(value: &RejectActivityRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for RejectActivityRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeRejectActivity => write!(f, "ACTIVITY_TYPE_REJECT_ACTIVITY"),
            }
        }
    }

    impl std::str::FromStr for RejectActivityRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_REJECT_ACTIVITY" => Ok(Self::ActivityTypeRejectActivity),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for RejectActivityRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for RejectActivityRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for RejectActivityRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///RemoveOrganizationFeatureIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "name"
    ///  ],
    ///  "properties": {
    ///    "name": {
    ///      "$ref": "#/components/schemas/FeatureName"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct RemoveOrganizationFeatureIntent {
        pub name: FeatureName,
    }

    impl From<&RemoveOrganizationFeatureIntent> for RemoveOrganizationFeatureIntent {
        fn from(value: &RemoveOrganizationFeatureIntent) -> Self {
            value.clone()
        }
    }

    ///RemoveOrganizationFeatureRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/RemoveOrganizationFeatureIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_REMOVE_ORGANIZATION_FEATURE"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct RemoveOrganizationFeatureRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: RemoveOrganizationFeatureIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: RemoveOrganizationFeatureRequestType,
    }

    impl From<&RemoveOrganizationFeatureRequest> for RemoveOrganizationFeatureRequest {
        fn from(value: &RemoveOrganizationFeatureRequest) -> Self {
            value.clone()
        }
    }

    ///RemoveOrganizationFeatureRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_REMOVE_ORGANIZATION_FEATURE"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum RemoveOrganizationFeatureRequestType {
        #[serde(rename = "ACTIVITY_TYPE_REMOVE_ORGANIZATION_FEATURE")]
        ActivityTypeRemoveOrganizationFeature,
    }

    impl From<&RemoveOrganizationFeatureRequestType> for RemoveOrganizationFeatureRequestType {
        fn from(value: &RemoveOrganizationFeatureRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for RemoveOrganizationFeatureRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeRemoveOrganizationFeature => {
                    write!(f, "ACTIVITY_TYPE_REMOVE_ORGANIZATION_FEATURE")
                }
            }
        }
    }

    impl std::str::FromStr for RemoveOrganizationFeatureRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_REMOVE_ORGANIZATION_FEATURE" => {
                    Ok(Self::ActivityTypeRemoveOrganizationFeature)
                }
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for RemoveOrganizationFeatureRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for RemoveOrganizationFeatureRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for RemoveOrganizationFeatureRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///RemoveOrganizationFeatureResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "features"
    ///  ],
    ///  "properties": {
    ///    "features": {
    ///      "description": "Resulting list of organization features.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/Feature"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct RemoveOrganizationFeatureResult {
        ///Resulting list of organization features.
        pub features: Vec<Feature>,
    }

    impl From<&RemoveOrganizationFeatureResult> for RemoveOrganizationFeatureResult {
        fn from(value: &RemoveOrganizationFeatureResult) -> Self {
            value.clone()
        }
    }

    ///Result
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "acceptInvitationResult": {
    ///      "$ref": "#/components/schemas/AcceptInvitationResult"
    ///    },
    ///    "activateBillingTierResult": {
    ///      "$ref": "#/components/schemas/ActivateBillingTierResult"
    ///    },
    ///    "createApiKeysResult": {
    ///      "$ref": "#/components/schemas/CreateApiKeysResult"
    ///    },
    ///    "createApiOnlyUsersResult": {
    ///      "$ref": "#/components/schemas/CreateApiOnlyUsersResult"
    ///    },
    ///    "createAuthenticatorsResult": {
    ///      "$ref": "#/components/schemas/CreateAuthenticatorsResult"
    ///    },
    ///    "createInvitationsResult": {
    ///      "$ref": "#/components/schemas/CreateInvitationsResult"
    ///    },
    ///    "createOauthProvidersResult": {
    ///      "$ref": "#/components/schemas/CreateOauthProvidersResult"
    ///    },
    ///    "createOrganizationResult": {
    ///      "$ref": "#/components/schemas/CreateOrganizationResult"
    ///    },
    ///    "createPoliciesResult": {
    ///      "$ref": "#/components/schemas/CreatePoliciesResult"
    ///    },
    ///    "createPolicyResult": {
    ///      "$ref": "#/components/schemas/CreatePolicyResult"
    ///    },
    ///    "createPrivateKeyTagResult": {
    ///      "$ref": "#/components/schemas/CreatePrivateKeyTagResult"
    ///    },
    ///    "createPrivateKeysResult": {
    ///      "$ref": "#/components/schemas/CreatePrivateKeysResult"
    ///    },
    ///    "createPrivateKeysResultV2": {
    ///      "$ref": "#/components/schemas/CreatePrivateKeysResultV2"
    ///    },
    ///    "createReadOnlySessionResult": {
    ///      "$ref": "#/components/schemas/CreateReadOnlySessionResult"
    ///    },
    ///    "createReadWriteSessionResult": {
    ///      "$ref": "#/components/schemas/CreateReadWriteSessionResult"
    ///    },
    ///    "createReadWriteSessionResultV2": {
    ///      "$ref": "#/components/schemas/CreateReadWriteSessionResultV2"
    ///    },
    ///    "createSubOrganizationResult": {
    ///      "$ref": "#/components/schemas/CreateSubOrganizationResult"
    ///    },
    ///    "createSubOrganizationResultV3": {
    ///      "$ref": "#/components/schemas/CreateSubOrganizationResultV3"
    ///    },
    ///    "createSubOrganizationResultV4": {
    ///      "$ref": "#/components/schemas/CreateSubOrganizationResultV4"
    ///    },
    ///    "createSubOrganizationResultV5": {
    ///      "$ref": "#/components/schemas/CreateSubOrganizationResultV5"
    ///    },
    ///    "createSubOrganizationResultV6": {
    ///      "$ref": "#/components/schemas/CreateSubOrganizationResultV6"
    ///    },
    ///    "createSubOrganizationResultV7": {
    ///      "$ref": "#/components/schemas/CreateSubOrganizationResultV7"
    ///    },
    ///    "createUserTagResult": {
    ///      "$ref": "#/components/schemas/CreateUserTagResult"
    ///    },
    ///    "createUsersResult": {
    ///      "$ref": "#/components/schemas/CreateUsersResult"
    ///    },
    ///    "createWalletAccountsResult": {
    ///      "$ref": "#/components/schemas/CreateWalletAccountsResult"
    ///    },
    ///    "createWalletResult": {
    ///      "$ref": "#/components/schemas/CreateWalletResult"
    ///    },
    ///    "deleteApiKeysResult": {
    ///      "$ref": "#/components/schemas/DeleteApiKeysResult"
    ///    },
    ///    "deleteAuthenticatorsResult": {
    ///      "$ref": "#/components/schemas/DeleteAuthenticatorsResult"
    ///    },
    ///    "deleteInvitationResult": {
    ///      "$ref": "#/components/schemas/DeleteInvitationResult"
    ///    },
    ///    "deleteOauthProvidersResult": {
    ///      "$ref": "#/components/schemas/DeleteOauthProvidersResult"
    ///    },
    ///    "deleteOrganizationResult": {
    ///      "$ref": "#/components/schemas/DeleteOrganizationResult"
    ///    },
    ///    "deletePaymentMethodResult": {
    ///      "$ref": "#/components/schemas/DeletePaymentMethodResult"
    ///    },
    ///    "deletePolicyResult": {
    ///      "$ref": "#/components/schemas/DeletePolicyResult"
    ///    },
    ///    "deletePrivateKeyTagsResult": {
    ///      "$ref": "#/components/schemas/DeletePrivateKeyTagsResult"
    ///    },
    ///    "deletePrivateKeysResult": {
    ///      "$ref": "#/components/schemas/DeletePrivateKeysResult"
    ///    },
    ///    "deleteSubOrganizationResult": {
    ///      "$ref": "#/components/schemas/DeleteSubOrganizationResult"
    ///    },
    ///    "deleteUserTagsResult": {
    ///      "$ref": "#/components/schemas/DeleteUserTagsResult"
    ///    },
    ///    "deleteUsersResult": {
    ///      "$ref": "#/components/schemas/DeleteUsersResult"
    ///    },
    ///    "deleteWalletsResult": {
    ///      "$ref": "#/components/schemas/DeleteWalletsResult"
    ///    },
    ///    "disablePrivateKeyResult": {
    ///      "$ref": "#/components/schemas/DisablePrivateKeyResult"
    ///    },
    ///    "emailAuthResult": {
    ///      "$ref": "#/components/schemas/EmailAuthResult"
    ///    },
    ///    "exportPrivateKeyResult": {
    ///      "$ref": "#/components/schemas/ExportPrivateKeyResult"
    ///    },
    ///    "exportWalletAccountResult": {
    ///      "$ref": "#/components/schemas/ExportWalletAccountResult"
    ///    },
    ///    "exportWalletResult": {
    ///      "$ref": "#/components/schemas/ExportWalletResult"
    ///    },
    ///    "importPrivateKeyResult": {
    ///      "$ref": "#/components/schemas/ImportPrivateKeyResult"
    ///    },
    ///    "importWalletResult": {
    ///      "$ref": "#/components/schemas/ImportWalletResult"
    ///    },
    ///    "initImportPrivateKeyResult": {
    ///      "$ref": "#/components/schemas/InitImportPrivateKeyResult"
    ///    },
    ///    "initImportWalletResult": {
    ///      "$ref": "#/components/schemas/InitImportWalletResult"
    ///    },
    ///    "initOtpAuthResult": {
    ///      "$ref": "#/components/schemas/InitOtpAuthResult"
    ///    },
    ///    "initUserEmailRecoveryResult": {
    ///      "$ref": "#/components/schemas/InitUserEmailRecoveryResult"
    ///    },
    ///    "oauthResult": {
    ///      "$ref": "#/components/schemas/OauthResult"
    ///    },
    ///    "otpAuthResult": {
    ///      "$ref": "#/components/schemas/OtpAuthResult"
    ///    },
    ///    "recoverUserResult": {
    ///      "$ref": "#/components/schemas/RecoverUserResult"
    ///    },
    ///    "removeOrganizationFeatureResult": {
    ///      "$ref": "#/components/schemas/RemoveOrganizationFeatureResult"
    ///    },
    ///    "setOrganizationFeatureResult": {
    ///      "$ref": "#/components/schemas/SetOrganizationFeatureResult"
    ///    },
    ///    "setPaymentMethodResult": {
    ///      "$ref": "#/components/schemas/SetPaymentMethodResult"
    ///    },
    ///    "signRawPayloadResult": {
    ///      "$ref": "#/components/schemas/SignRawPayloadResult"
    ///    },
    ///    "signRawPayloadsResult": {
    ///      "$ref": "#/components/schemas/SignRawPayloadsResult"
    ///    },
    ///    "signTransactionResult": {
    ///      "$ref": "#/components/schemas/SignTransactionResult"
    ///    },
    ///    "updateAllowedOriginsResult": {
    ///      "$ref": "#/components/schemas/UpdateAllowedOriginsResult"
    ///    },
    ///    "updatePolicyResult": {
    ///      "$ref": "#/components/schemas/UpdatePolicyResult"
    ///    },
    ///    "updatePrivateKeyTagResult": {
    ///      "$ref": "#/components/schemas/UpdatePrivateKeyTagResult"
    ///    },
    ///    "updateRootQuorumResult": {
    ///      "$ref": "#/components/schemas/UpdateRootQuorumResult"
    ///    },
    ///    "updateUserResult": {
    ///      "$ref": "#/components/schemas/UpdateUserResult"
    ///    },
    ///    "updateUserTagResult": {
    ///      "$ref": "#/components/schemas/UpdateUserTagResult"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct Result {
        #[serde(
            rename = "acceptInvitationResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub accept_invitation_result: Option<AcceptInvitationResult>,
        #[serde(
            rename = "activateBillingTierResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub activate_billing_tier_result: Option<ActivateBillingTierResult>,
        #[serde(
            rename = "createApiKeysResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_api_keys_result: Option<CreateApiKeysResult>,
        #[serde(
            rename = "createApiOnlyUsersResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_api_only_users_result: Option<CreateApiOnlyUsersResult>,
        #[serde(
            rename = "createAuthenticatorsResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_authenticators_result: Option<CreateAuthenticatorsResult>,
        #[serde(
            rename = "createInvitationsResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_invitations_result: Option<CreateInvitationsResult>,
        #[serde(
            rename = "createOauthProvidersResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_oauth_providers_result: Option<CreateOauthProvidersResult>,
        #[serde(
            rename = "createOrganizationResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_organization_result: Option<CreateOrganizationResult>,
        #[serde(
            rename = "createPoliciesResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_policies_result: Option<CreatePoliciesResult>,
        #[serde(
            rename = "createPolicyResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_policy_result: Option<CreatePolicyResult>,
        #[serde(
            rename = "createPrivateKeyTagResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_private_key_tag_result: Option<CreatePrivateKeyTagResult>,
        #[serde(
            rename = "createPrivateKeysResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_private_keys_result: Option<CreatePrivateKeysResult>,
        #[serde(
            rename = "createPrivateKeysResultV2",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_private_keys_result_v2: Option<CreatePrivateKeysResultV2>,
        #[serde(
            rename = "createReadOnlySessionResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_read_only_session_result: Option<CreateReadOnlySessionResult>,
        #[serde(
            rename = "createReadWriteSessionResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_read_write_session_result: Option<CreateReadWriteSessionResult>,
        #[serde(
            rename = "createReadWriteSessionResultV2",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_read_write_session_result_v2: Option<CreateReadWriteSessionResultV2>,
        #[serde(
            rename = "createSubOrganizationResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_sub_organization_result: Option<CreateSubOrganizationResult>,
        #[serde(
            rename = "createSubOrganizationResultV3",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_sub_organization_result_v3: Option<CreateSubOrganizationResultV3>,
        #[serde(
            rename = "createSubOrganizationResultV4",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_sub_organization_result_v4: Option<CreateSubOrganizationResultV4>,
        #[serde(
            rename = "createSubOrganizationResultV5",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_sub_organization_result_v5: Option<CreateSubOrganizationResultV5>,
        #[serde(
            rename = "createSubOrganizationResultV6",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_sub_organization_result_v6: Option<CreateSubOrganizationResultV6>,
        #[serde(
            rename = "createSubOrganizationResultV7",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_sub_organization_result_v7: Option<CreateSubOrganizationResultV7>,
        #[serde(
            rename = "createUserTagResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_user_tag_result: Option<CreateUserTagResult>,
        #[serde(
            rename = "createUsersResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_users_result: Option<CreateUsersResult>,
        #[serde(
            rename = "createWalletAccountsResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_wallet_accounts_result: Option<CreateWalletAccountsResult>,
        #[serde(
            rename = "createWalletResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub create_wallet_result: Option<CreateWalletResult>,
        #[serde(
            rename = "deleteApiKeysResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_api_keys_result: Option<DeleteApiKeysResult>,
        #[serde(
            rename = "deleteAuthenticatorsResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_authenticators_result: Option<DeleteAuthenticatorsResult>,
        #[serde(
            rename = "deleteInvitationResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_invitation_result: Option<DeleteInvitationResult>,
        #[serde(
            rename = "deleteOauthProvidersResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_oauth_providers_result: Option<DeleteOauthProvidersResult>,
        #[serde(
            rename = "deleteOrganizationResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_organization_result: Option<DeleteOrganizationResult>,
        #[serde(
            rename = "deletePaymentMethodResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_payment_method_result: Option<DeletePaymentMethodResult>,
        #[serde(
            rename = "deletePolicyResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_policy_result: Option<DeletePolicyResult>,
        #[serde(
            rename = "deletePrivateKeyTagsResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_private_key_tags_result: Option<DeletePrivateKeyTagsResult>,
        #[serde(
            rename = "deletePrivateKeysResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_private_keys_result: Option<DeletePrivateKeysResult>,
        #[serde(
            rename = "deleteSubOrganizationResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_sub_organization_result: Option<DeleteSubOrganizationResult>,
        #[serde(
            rename = "deleteUserTagsResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_user_tags_result: Option<DeleteUserTagsResult>,
        #[serde(
            rename = "deleteUsersResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_users_result: Option<DeleteUsersResult>,
        #[serde(
            rename = "deleteWalletsResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub delete_wallets_result: Option<DeleteWalletsResult>,
        #[serde(
            rename = "disablePrivateKeyResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub disable_private_key_result: Option<DisablePrivateKeyResult>,
        #[serde(
            rename = "emailAuthResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub email_auth_result: Option<EmailAuthResult>,
        #[serde(
            rename = "exportPrivateKeyResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub export_private_key_result: Option<ExportPrivateKeyResult>,
        #[serde(
            rename = "exportWalletAccountResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub export_wallet_account_result: Option<ExportWalletAccountResult>,
        #[serde(
            rename = "exportWalletResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub export_wallet_result: Option<ExportWalletResult>,
        #[serde(
            rename = "importPrivateKeyResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub import_private_key_result: Option<ImportPrivateKeyResult>,
        #[serde(
            rename = "importWalletResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub import_wallet_result: Option<ImportWalletResult>,
        #[serde(
            rename = "initImportPrivateKeyResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub init_import_private_key_result: Option<InitImportPrivateKeyResult>,
        #[serde(
            rename = "initImportWalletResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub init_import_wallet_result: Option<InitImportWalletResult>,
        #[serde(
            rename = "initOtpAuthResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub init_otp_auth_result: Option<InitOtpAuthResult>,
        #[serde(
            rename = "initUserEmailRecoveryResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub init_user_email_recovery_result: Option<InitUserEmailRecoveryResult>,
        #[serde(
            rename = "oauthResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub oauth_result: Option<OauthResult>,
        #[serde(
            rename = "otpAuthResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub otp_auth_result: Option<OtpAuthResult>,
        #[serde(
            rename = "recoverUserResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub recover_user_result: Option<RecoverUserResult>,
        #[serde(
            rename = "removeOrganizationFeatureResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub remove_organization_feature_result: Option<RemoveOrganizationFeatureResult>,
        #[serde(
            rename = "setOrganizationFeatureResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub set_organization_feature_result: Option<SetOrganizationFeatureResult>,
        #[serde(
            rename = "setPaymentMethodResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub set_payment_method_result: Option<SetPaymentMethodResult>,
        #[serde(
            rename = "signRawPayloadResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub sign_raw_payload_result: Option<SignRawPayloadResult>,
        #[serde(
            rename = "signRawPayloadsResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub sign_raw_payloads_result: Option<SignRawPayloadsResult>,
        #[serde(
            rename = "signTransactionResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub sign_transaction_result: Option<SignTransactionResult>,
        #[serde(
            rename = "updateAllowedOriginsResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub update_allowed_origins_result: Option<UpdateAllowedOriginsResult>,
        #[serde(
            rename = "updatePolicyResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub update_policy_result: Option<UpdatePolicyResult>,
        #[serde(
            rename = "updatePrivateKeyTagResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub update_private_key_tag_result: Option<UpdatePrivateKeyTagResult>,
        #[serde(
            rename = "updateRootQuorumResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub update_root_quorum_result: Option<UpdateRootQuorumResult>,
        #[serde(
            rename = "updateUserResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub update_user_result: Option<UpdateUserResult>,
        #[serde(
            rename = "updateUserTagResult",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub update_user_tag_result: Option<UpdateUserTagResult>,
    }

    impl From<&Result> for Result {
        fn from(value: &Result) -> Self {
            value.clone()
        }
    }

    ///RootUserParams
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "apiKeys",
    ///    "authenticators",
    ///    "userName"
    ///  ],
    ///  "properties": {
    ///    "apiKeys": {
    ///      "description": "A list of API Key parameters.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/ApiKeyParams"
    ///      }
    ///    },
    ///    "authenticators": {
    ///      "description": "A list of Authenticator parameters.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/AuthenticatorParamsV2"
    ///      }
    ///    },
    ///    "userEmail": {
    ///      "description": "The user's email address.",
    ///      "type": "string"
    ///    },
    ///    "userName": {
    ///      "description": "Human-readable name for a User.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct RootUserParams {
        ///A list of API Key parameters.
        #[serde(rename = "apiKeys")]
        pub api_keys: Vec<ApiKeyParams>,
        ///A list of Authenticator parameters.
        pub authenticators: Vec<AuthenticatorParamsV2>,
        ///The user's email address.
        #[serde(rename = "userEmail", default, skip_serializing_if = "Option::is_none")]
        pub user_email: Option<String>,
        ///Human-readable name for a User.
        #[serde(rename = "userName")]
        pub user_name: String,
    }

    impl From<&RootUserParams> for RootUserParams {
        fn from(value: &RootUserParams) -> Self {
            value.clone()
        }
    }

    ///RootUserParamsV2
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "apiKeys",
    ///    "authenticators",
    ///    "oauthProviders",
    ///    "userName"
    ///  ],
    ///  "properties": {
    ///    "apiKeys": {
    ///      "description": "A list of API Key parameters.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/ApiKeyParams"
    ///      }
    ///    },
    ///    "authenticators": {
    ///      "description": "A list of Authenticator parameters.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/AuthenticatorParamsV2"
    ///      }
    ///    },
    ///    "oauthProviders": {
    ///      "description": "A list of Oauth providers.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/OauthProviderParams"
    ///      }
    ///    },
    ///    "userEmail": {
    ///      "description": "The user's email address.",
    ///      "type": "string"
    ///    },
    ///    "userName": {
    ///      "description": "Human-readable name for a User.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct RootUserParamsV2 {
        ///A list of API Key parameters.
        #[serde(rename = "apiKeys")]
        pub api_keys: Vec<ApiKeyParams>,
        ///A list of Authenticator parameters.
        pub authenticators: Vec<AuthenticatorParamsV2>,
        ///A list of Oauth providers.
        #[serde(rename = "oauthProviders")]
        pub oauth_providers: Vec<OauthProviderParams>,
        ///The user's email address.
        #[serde(rename = "userEmail", default, skip_serializing_if = "Option::is_none")]
        pub user_email: Option<String>,
        ///Human-readable name for a User.
        #[serde(rename = "userName")]
        pub user_name: String,
    }

    impl From<&RootUserParamsV2> for RootUserParamsV2 {
        fn from(value: &RootUserParamsV2) -> Self {
            value.clone()
        }
    }

    ///RootUserParamsV3
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "apiKeys",
    ///    "authenticators",
    ///    "oauthProviders",
    ///    "userName"
    ///  ],
    ///  "properties": {
    ///    "apiKeys": {
    ///      "description": "A list of API Key parameters.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/ApiKeyParamsV2"
    ///      }
    ///    },
    ///    "authenticators": {
    ///      "description": "A list of Authenticator parameters.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/AuthenticatorParamsV2"
    ///      }
    ///    },
    ///    "oauthProviders": {
    ///      "description": "A list of Oauth providers.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/OauthProviderParams"
    ///      }
    ///    },
    ///    "userEmail": {
    ///      "description": "The user's email address.",
    ///      "type": "string"
    ///    },
    ///    "userName": {
    ///      "description": "Human-readable name for a User.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct RootUserParamsV3 {
        ///A list of API Key parameters.
        #[serde(rename = "apiKeys")]
        pub api_keys: Vec<ApiKeyParamsV2>,
        ///A list of Authenticator parameters.
        pub authenticators: Vec<AuthenticatorParamsV2>,
        ///A list of Oauth providers.
        #[serde(rename = "oauthProviders")]
        pub oauth_providers: Vec<OauthProviderParams>,
        ///The user's email address.
        #[serde(rename = "userEmail", default, skip_serializing_if = "Option::is_none")]
        pub user_email: Option<String>,
        ///Human-readable name for a User.
        #[serde(rename = "userName")]
        pub user_name: String,
    }

    impl From<&RootUserParamsV3> for RootUserParamsV3 {
        fn from(value: &RootUserParamsV3) -> Self {
            value.clone()
        }
    }

    ///RootUserParamsV4
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "apiKeys",
    ///    "authenticators",
    ///    "oauthProviders",
    ///    "userName"
    ///  ],
    ///  "properties": {
    ///    "apiKeys": {
    ///      "description": "A list of API Key parameters.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/ApiKeyParamsV2"
    ///      }
    ///    },
    ///    "authenticators": {
    ///      "description": "A list of Authenticator parameters.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/AuthenticatorParamsV2"
    ///      }
    ///    },
    ///    "oauthProviders": {
    ///      "description": "A list of Oauth providers.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/OauthProviderParams"
    ///      }
    ///    },
    ///    "userEmail": {
    ///      "description": "The user's email address.",
    ///      "type": "string"
    ///    },
    ///    "userName": {
    ///      "description": "Human-readable name for a User.",
    ///      "type": "string"
    ///    },
    ///    "userPhoneNumber": {
    ///      "description": "The user's phone number in E.164 format e.g.
    /// +13214567890",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct RootUserParamsV4 {
        ///A list of API Key parameters.
        #[serde(rename = "apiKeys")]
        pub api_keys: Vec<ApiKeyParamsV2>,
        ///A list of Authenticator parameters.
        pub authenticators: Vec<AuthenticatorParamsV2>,
        ///A list of Oauth providers.
        #[serde(rename = "oauthProviders")]
        pub oauth_providers: Vec<OauthProviderParams>,
        ///The user's email address.
        #[serde(rename = "userEmail", default, skip_serializing_if = "Option::is_none")]
        pub user_email: Option<String>,
        ///Human-readable name for a User.
        #[serde(rename = "userName")]
        pub user_name: String,
        ///The user's phone number in E.164 format e.g. +13214567890
        #[serde(
            rename = "userPhoneNumber",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub user_phone_number: Option<String>,
    }

    impl From<&RootUserParamsV4> for RootUserParamsV4 {
        fn from(value: &RootUserParamsV4) -> Self {
            value.clone()
        }
    }

    ///Selector
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "operator": {
    ///      "$ref": "#/components/schemas/Operator"
    ///    },
    ///    "subject": {
    ///      "type": "string"
    ///    },
    ///    "target": {
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct Selector {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub operator: Option<Operator>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub subject: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub target: Option<String>,
    }

    impl From<&Selector> for Selector {
        fn from(value: &Selector) -> Self {
            value.clone()
        }
    }

    ///SelectorV2
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "operator": {
    ///      "$ref": "#/components/schemas/Operator"
    ///    },
    ///    "subject": {
    ///      "type": "string"
    ///    },
    ///    "targets": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct SelectorV2 {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub operator: Option<Operator>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub subject: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub targets: Vec<String>,
    }

    impl From<&SelectorV2> for SelectorV2 {
        fn from(value: &SelectorV2) -> Self {
            value.clone()
        }
    }

    ///SetOrganizationFeatureIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "name",
    ///    "value"
    ///  ],
    ///  "properties": {
    ///    "name": {
    ///      "$ref": "#/components/schemas/FeatureName"
    ///    },
    ///    "value": {
    ///      "description": "Optional value for the feature. Will override
    /// existing values if feature is already set.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct SetOrganizationFeatureIntent {
        pub name: FeatureName,
        ///Optional value for the feature. Will override existing values if
        /// feature is already set.
        pub value: String,
    }

    impl From<&SetOrganizationFeatureIntent> for SetOrganizationFeatureIntent {
        fn from(value: &SetOrganizationFeatureIntent) -> Self {
            value.clone()
        }
    }

    ///SetOrganizationFeatureRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/SetOrganizationFeatureIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_SET_ORGANIZATION_FEATURE"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct SetOrganizationFeatureRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: SetOrganizationFeatureIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: SetOrganizationFeatureRequestType,
    }

    impl From<&SetOrganizationFeatureRequest> for SetOrganizationFeatureRequest {
        fn from(value: &SetOrganizationFeatureRequest) -> Self {
            value.clone()
        }
    }

    ///SetOrganizationFeatureRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_SET_ORGANIZATION_FEATURE"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum SetOrganizationFeatureRequestType {
        #[serde(rename = "ACTIVITY_TYPE_SET_ORGANIZATION_FEATURE")]
        ActivityTypeSetOrganizationFeature,
    }

    impl From<&SetOrganizationFeatureRequestType> for SetOrganizationFeatureRequestType {
        fn from(value: &SetOrganizationFeatureRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for SetOrganizationFeatureRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeSetOrganizationFeature => {
                    write!(f, "ACTIVITY_TYPE_SET_ORGANIZATION_FEATURE")
                }
            }
        }
    }

    impl std::str::FromStr for SetOrganizationFeatureRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_SET_ORGANIZATION_FEATURE" => {
                    Ok(Self::ActivityTypeSetOrganizationFeature)
                }
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for SetOrganizationFeatureRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for SetOrganizationFeatureRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for SetOrganizationFeatureRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///SetOrganizationFeatureResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "features"
    ///  ],
    ///  "properties": {
    ///    "features": {
    ///      "description": "Resulting list of organization features.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/Feature"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct SetOrganizationFeatureResult {
        ///Resulting list of organization features.
        pub features: Vec<Feature>,
    }

    impl From<&SetOrganizationFeatureResult> for SetOrganizationFeatureResult {
        fn from(value: &SetOrganizationFeatureResult) -> Self {
            value.clone()
        }
    }

    ///SetPaymentMethodIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "cardHolderEmail",
    ///    "cardHolderName",
    ///    "cvv",
    ///    "expiryMonth",
    ///    "expiryYear",
    ///    "number"
    ///  ],
    ///  "properties": {
    ///    "cardHolderEmail": {
    ///      "description": "The email that will receive invoices for the credit
    /// card.",
    ///      "type": "string"
    ///    },
    ///    "cardHolderName": {
    ///      "description": "The name associated with the credit card.",
    ///      "type": "string"
    ///    },
    ///    "cvv": {
    ///      "description": "The verification digits of the customer's credit
    /// card.",
    ///      "type": "string"
    ///    },
    ///    "expiryMonth": {
    ///      "description": "The month that the credit card expires.",
    ///      "type": "string"
    ///    },
    ///    "expiryYear": {
    ///      "description": "The year that the credit card expires.",
    ///      "type": "string"
    ///    },
    ///    "number": {
    ///      "description": "The account number of the customer's credit card.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct SetPaymentMethodIntent {
        ///The email that will receive invoices for the credit card.
        #[serde(rename = "cardHolderEmail")]
        pub card_holder_email: String,
        ///The name associated with the credit card.
        #[serde(rename = "cardHolderName")]
        pub card_holder_name: String,
        ///The verification digits of the customer's credit card.
        pub cvv: String,
        ///The month that the credit card expires.
        #[serde(rename = "expiryMonth")]
        pub expiry_month: String,
        ///The year that the credit card expires.
        #[serde(rename = "expiryYear")]
        pub expiry_year: String,
        ///The account number of the customer's credit card.
        pub number: String,
    }

    impl From<&SetPaymentMethodIntent> for SetPaymentMethodIntent {
        fn from(value: &SetPaymentMethodIntent) -> Self {
            value.clone()
        }
    }

    ///SetPaymentMethodIntentV2
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "cardHolderEmail",
    ///    "cardHolderName",
    ///    "paymentMethodId"
    ///  ],
    ///  "properties": {
    ///    "cardHolderEmail": {
    ///      "description": "The email that will receive invoices for the credit
    /// card.",
    ///      "type": "string"
    ///    },
    ///    "cardHolderName": {
    ///      "description": "The name associated with the credit card.",
    ///      "type": "string"
    ///    },
    ///    "paymentMethodId": {
    ///      "description": "The id of the payment method that was created
    /// clientside.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct SetPaymentMethodIntentV2 {
        ///The email that will receive invoices for the credit card.
        #[serde(rename = "cardHolderEmail")]
        pub card_holder_email: String,
        ///The name associated with the credit card.
        #[serde(rename = "cardHolderName")]
        pub card_holder_name: String,
        ///The id of the payment method that was created clientside.
        #[serde(rename = "paymentMethodId")]
        pub payment_method_id: String,
    }

    impl From<&SetPaymentMethodIntentV2> for SetPaymentMethodIntentV2 {
        fn from(value: &SetPaymentMethodIntentV2) -> Self {
            value.clone()
        }
    }

    ///SetPaymentMethodResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "cardHolderEmail",
    ///    "cardHolderName",
    ///    "lastFour"
    ///  ],
    ///  "properties": {
    ///    "cardHolderEmail": {
    ///      "description": "The email address associated with the payment
    /// method.",
    ///      "type": "string"
    ///    },
    ///    "cardHolderName": {
    ///      "description": "The name associated with the payment method.",
    ///      "type": "string"
    ///    },
    ///    "lastFour": {
    ///      "description": "The last four digits of the credit card added.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct SetPaymentMethodResult {
        ///The email address associated with the payment method.
        #[serde(rename = "cardHolderEmail")]
        pub card_holder_email: String,
        ///The name associated with the payment method.
        #[serde(rename = "cardHolderName")]
        pub card_holder_name: String,
        ///The last four digits of the credit card added.
        #[serde(rename = "lastFour")]
        pub last_four: String,
    }

    impl From<&SetPaymentMethodResult> for SetPaymentMethodResult {
        fn from(value: &SetPaymentMethodResult) -> Self {
            value.clone()
        }
    }

    ///SignRawPayloadIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "encoding",
    ///    "hashFunction",
    ///    "payload",
    ///    "privateKeyId"
    ///  ],
    ///  "properties": {
    ///    "encoding": {
    ///      "$ref": "#/components/schemas/PayloadEncoding"
    ///    },
    ///    "hashFunction": {
    ///      "$ref": "#/components/schemas/HashFunction"
    ///    },
    ///    "payload": {
    ///      "description": "Raw unsigned payload to be signed.",
    ///      "type": "string"
    ///    },
    ///    "privateKeyId": {
    ///      "description": "Unique identifier for a given Private Key.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct SignRawPayloadIntent {
        pub encoding: PayloadEncoding,
        #[serde(rename = "hashFunction")]
        pub hash_function: HashFunction,
        ///Raw unsigned payload to be signed.
        pub payload: String,
        ///Unique identifier for a given Private Key.
        #[serde(rename = "privateKeyId")]
        pub private_key_id: String,
    }

    impl From<&SignRawPayloadIntent> for SignRawPayloadIntent {
        fn from(value: &SignRawPayloadIntent) -> Self {
            value.clone()
        }
    }

    ///SignRawPayloadIntentV2
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "encoding",
    ///    "hashFunction",
    ///    "payload",
    ///    "signWith"
    ///  ],
    ///  "properties": {
    ///    "encoding": {
    ///      "$ref": "#/components/schemas/PayloadEncoding"
    ///    },
    ///    "hashFunction": {
    ///      "$ref": "#/components/schemas/HashFunction"
    ///    },
    ///    "payload": {
    ///      "description": "Raw unsigned payload to be signed.",
    ///      "type": "string"
    ///    },
    ///    "signWith": {
    ///      "description": "A Wallet account address, Private Key address, or
    /// Private Key identifier.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct SignRawPayloadIntentV2 {
        pub encoding: PayloadEncoding,
        #[serde(rename = "hashFunction")]
        pub hash_function: HashFunction,
        ///Raw unsigned payload to be signed.
        pub payload: String,
        ///A Wallet account address, Private Key address, or Private Key
        /// identifier.
        #[serde(rename = "signWith")]
        pub sign_with: String,
    }

    impl From<&SignRawPayloadIntentV2> for SignRawPayloadIntentV2 {
        fn from(value: &SignRawPayloadIntentV2) -> Self {
            value.clone()
        }
    }

    ///SignRawPayloadRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/SignRawPayloadIntentV2"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_SIGN_RAW_PAYLOAD_V2"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct SignRawPayloadRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: SignRawPayloadIntentV2,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: SignRawPayloadRequestType,
    }

    impl From<&SignRawPayloadRequest> for SignRawPayloadRequest {
        fn from(value: &SignRawPayloadRequest) -> Self {
            value.clone()
        }
    }

    ///SignRawPayloadRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_SIGN_RAW_PAYLOAD_V2"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum SignRawPayloadRequestType {
        #[serde(rename = "ACTIVITY_TYPE_SIGN_RAW_PAYLOAD_V2")]
        ActivityTypeSignRawPayloadV2,
    }

    impl From<&SignRawPayloadRequestType> for SignRawPayloadRequestType {
        fn from(value: &SignRawPayloadRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for SignRawPayloadRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeSignRawPayloadV2 => {
                    write!(f, "ACTIVITY_TYPE_SIGN_RAW_PAYLOAD_V2")
                }
            }
        }
    }

    impl std::str::FromStr for SignRawPayloadRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_SIGN_RAW_PAYLOAD_V2" => Ok(Self::ActivityTypeSignRawPayloadV2),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for SignRawPayloadRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for SignRawPayloadRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for SignRawPayloadRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///SignRawPayloadResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "r",
    ///    "s",
    ///    "v"
    ///  ],
    ///  "properties": {
    ///    "r": {
    ///      "description": "Component of an ECSDA signature.",
    ///      "type": "string"
    ///    },
    ///    "s": {
    ///      "description": "Component of an ECSDA signature.",
    ///      "type": "string"
    ///    },
    ///    "v": {
    ///      "description": "Component of an ECSDA signature.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct SignRawPayloadResult {
        ///Component of an ECSDA signature.
        pub r: String,
        ///Component of an ECSDA signature.
        pub s: String,
        ///Component of an ECSDA signature.
        pub v: String,
    }

    impl From<&SignRawPayloadResult> for SignRawPayloadResult {
        fn from(value: &SignRawPayloadResult) -> Self {
            value.clone()
        }
    }

    ///SignRawPayloadsIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "encoding",
    ///    "hashFunction",
    ///    "payloads",
    ///    "signWith"
    ///  ],
    ///  "properties": {
    ///    "encoding": {
    ///      "$ref": "#/components/schemas/PayloadEncoding"
    ///    },
    ///    "hashFunction": {
    ///      "$ref": "#/components/schemas/HashFunction"
    ///    },
    ///    "payloads": {
    ///      "description": "An array of raw unsigned payloads to be signed.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "signWith": {
    ///      "description": "A Wallet account address, Private Key address, or
    /// Private Key identifier.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct SignRawPayloadsIntent {
        pub encoding: PayloadEncoding,
        #[serde(rename = "hashFunction")]
        pub hash_function: HashFunction,
        ///An array of raw unsigned payloads to be signed.
        pub payloads: Vec<String>,
        ///A Wallet account address, Private Key address, or Private Key
        /// identifier.
        #[serde(rename = "signWith")]
        pub sign_with: String,
    }

    impl From<&SignRawPayloadsIntent> for SignRawPayloadsIntent {
        fn from(value: &SignRawPayloadsIntent) -> Self {
            value.clone()
        }
    }

    ///SignRawPayloadsRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/SignRawPayloadsIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_SIGN_RAW_PAYLOADS"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct SignRawPayloadsRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: SignRawPayloadsIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: SignRawPayloadsRequestType,
    }

    impl From<&SignRawPayloadsRequest> for SignRawPayloadsRequest {
        fn from(value: &SignRawPayloadsRequest) -> Self {
            value.clone()
        }
    }

    ///SignRawPayloadsRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_SIGN_RAW_PAYLOADS"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum SignRawPayloadsRequestType {
        #[serde(rename = "ACTIVITY_TYPE_SIGN_RAW_PAYLOADS")]
        ActivityTypeSignRawPayloads,
    }

    impl From<&SignRawPayloadsRequestType> for SignRawPayloadsRequestType {
        fn from(value: &SignRawPayloadsRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for SignRawPayloadsRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeSignRawPayloads => write!(f, "ACTIVITY_TYPE_SIGN_RAW_PAYLOADS"),
            }
        }
    }

    impl std::str::FromStr for SignRawPayloadsRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_SIGN_RAW_PAYLOADS" => Ok(Self::ActivityTypeSignRawPayloads),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for SignRawPayloadsRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for SignRawPayloadsRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for SignRawPayloadsRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///SignRawPayloadsResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "signatures": {
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/SignRawPayloadResult"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct SignRawPayloadsResult {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub signatures: Vec<SignRawPayloadResult>,
    }

    impl From<&SignRawPayloadsResult> for SignRawPayloadsResult {
        fn from(value: &SignRawPayloadsResult) -> Self {
            value.clone()
        }
    }

    ///SignTransactionIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "privateKeyId",
    ///    "type",
    ///    "unsignedTransaction"
    ///  ],
    ///  "properties": {
    ///    "privateKeyId": {
    ///      "description": "Unique identifier for a given Private Key.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "$ref": "#/components/schemas/TransactionType"
    ///    },
    ///    "unsignedTransaction": {
    ///      "description": "Raw unsigned transaction to be signed by a
    /// particular Private Key.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct SignTransactionIntent {
        ///Unique identifier for a given Private Key.
        #[serde(rename = "privateKeyId")]
        pub private_key_id: String,
        #[serde(rename = "type")]
        pub type_: TransactionType,
        ///Raw unsigned transaction to be signed by a particular Private Key.
        #[serde(rename = "unsignedTransaction")]
        pub unsigned_transaction: String,
    }

    impl From<&SignTransactionIntent> for SignTransactionIntent {
        fn from(value: &SignTransactionIntent) -> Self {
            value.clone()
        }
    }

    ///SignTransactionIntentV2
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "signWith",
    ///    "type",
    ///    "unsignedTransaction"
    ///  ],
    ///  "properties": {
    ///    "signWith": {
    ///      "description": "A Wallet account address, Private Key address, or
    /// Private Key identifier.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "$ref": "#/components/schemas/TransactionType"
    ///    },
    ///    "unsignedTransaction": {
    ///      "description": "Raw unsigned transaction to be signed",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct SignTransactionIntentV2 {
        ///A Wallet account address, Private Key address, or Private Key
        /// identifier.
        #[serde(rename = "signWith")]
        pub sign_with: String,
        #[serde(rename = "type")]
        pub type_: TransactionType,
        ///Raw unsigned transaction to be signed
        #[serde(rename = "unsignedTransaction")]
        pub unsigned_transaction: String,
    }

    impl From<&SignTransactionIntentV2> for SignTransactionIntentV2 {
        fn from(value: &SignTransactionIntentV2) -> Self {
            value.clone()
        }
    }

    ///SignTransactionRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/SignTransactionIntentV2"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_SIGN_TRANSACTION_V2"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct SignTransactionRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: SignTransactionIntentV2,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: SignTransactionRequestType,
    }

    impl From<&SignTransactionRequest> for SignTransactionRequest {
        fn from(value: &SignTransactionRequest) -> Self {
            value.clone()
        }
    }

    ///SignTransactionRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_SIGN_TRANSACTION_V2"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum SignTransactionRequestType {
        #[serde(rename = "ACTIVITY_TYPE_SIGN_TRANSACTION_V2")]
        ActivityTypeSignTransactionV2,
    }

    impl From<&SignTransactionRequestType> for SignTransactionRequestType {
        fn from(value: &SignTransactionRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for SignTransactionRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeSignTransactionV2 => {
                    write!(f, "ACTIVITY_TYPE_SIGN_TRANSACTION_V2")
                }
            }
        }
    }

    impl std::str::FromStr for SignTransactionRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_SIGN_TRANSACTION_V2" => Ok(Self::ActivityTypeSignTransactionV2),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for SignTransactionRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for SignTransactionRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for SignTransactionRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///SignTransactionResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "signedTransaction"
    ///  ],
    ///  "properties": {
    ///    "signedTransaction": {
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct SignTransactionResult {
        #[serde(rename = "signedTransaction")]
        pub signed_transaction: String,
    }

    impl From<&SignTransactionResult> for SignTransactionResult {
        fn from(value: &SignTransactionResult) -> Self {
            value.clone()
        }
    }

    ///SimpleClientExtensionResults
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "appid": {
    ///      "type": "boolean"
    ///    },
    ///    "appidExclude": {
    ///      "type": "boolean"
    ///    },
    ///    "credProps": {
    ///      "$ref":
    /// "#/components/schemas/CredPropsAuthenticationExtensionsClientOutputs"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct SimpleClientExtensionResults {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub appid: Option<bool>,
        #[serde(
            rename = "appidExclude",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub appid_exclude: Option<bool>,
        #[serde(rename = "credProps", default, skip_serializing_if = "Option::is_none")]
        pub cred_props: Option<CredPropsAuthenticationExtensionsClientOutputs>,
    }

    impl From<&SimpleClientExtensionResults> for SimpleClientExtensionResults {
        fn from(value: &SimpleClientExtensionResults) -> Self {
            value.clone()
        }
    }

    ///Status
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "code": {
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "details": {
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/Any"
    ///      }
    ///    },
    ///    "message": {
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct Status {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub code: Option<i32>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub details: Vec<Any>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub message: Option<String>,
    }

    impl From<&Status> for Status {
        fn from(value: &Status) -> Self {
            value.clone()
        }
    }

    ///TagType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "TAG_TYPE_USER",
    ///    "TAG_TYPE_PRIVATE_KEY"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum TagType {
        #[serde(rename = "TAG_TYPE_USER")]
        TagTypeUser,
        #[serde(rename = "TAG_TYPE_PRIVATE_KEY")]
        TagTypePrivateKey,
    }

    impl From<&TagType> for TagType {
        fn from(value: &TagType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for TagType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::TagTypeUser => write!(f, "TAG_TYPE_USER"),
                Self::TagTypePrivateKey => write!(f, "TAG_TYPE_PRIVATE_KEY"),
            }
        }
    }

    impl std::str::FromStr for TagType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "TAG_TYPE_USER" => Ok(Self::TagTypeUser),
                "TAG_TYPE_PRIVATE_KEY" => Ok(Self::TagTypePrivateKey),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for TagType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for TagType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for TagType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///TransactionType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "TRANSACTION_TYPE_ETHEREUM",
    ///    "TRANSACTION_TYPE_SOLANA"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum TransactionType {
        #[serde(rename = "TRANSACTION_TYPE_ETHEREUM")]
        TransactionTypeEthereum,
        #[serde(rename = "TRANSACTION_TYPE_SOLANA")]
        TransactionTypeSolana,
    }

    impl From<&TransactionType> for TransactionType {
        fn from(value: &TransactionType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for TransactionType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::TransactionTypeEthereum => write!(f, "TRANSACTION_TYPE_ETHEREUM"),
                Self::TransactionTypeSolana => write!(f, "TRANSACTION_TYPE_SOLANA"),
            }
        }
    }

    impl std::str::FromStr for TransactionType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "TRANSACTION_TYPE_ETHEREUM" => Ok(Self::TransactionTypeEthereum),
                "TRANSACTION_TYPE_SOLANA" => Ok(Self::TransactionTypeSolana),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for TransactionType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for TransactionType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for TransactionType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///UpdateAllowedOriginsIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "allowedOrigins"
    ///  ],
    ///  "properties": {
    ///    "allowedOrigins": {
    ///      "description": "Additional origins requests are allowed from
    /// besides Turnkey origins",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct UpdateAllowedOriginsIntent {
        ///Additional origins requests are allowed from besides Turnkey origins
        #[serde(rename = "allowedOrigins")]
        pub allowed_origins: Vec<String>,
    }

    impl From<&UpdateAllowedOriginsIntent> for UpdateAllowedOriginsIntent {
        fn from(value: &UpdateAllowedOriginsIntent) -> Self {
            value.clone()
        }
    }

    ///UpdateAllowedOriginsResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object"
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct UpdateAllowedOriginsResult(pub ::serde_json::Map<String, ::serde_json::Value>);
    impl ::std::ops::Deref for UpdateAllowedOriginsResult {
        type Target = ::serde_json::Map<String, ::serde_json::Value>;
        fn deref(&self) -> &::serde_json::Map<String, ::serde_json::Value> {
            &self.0
        }
    }

    impl From<UpdateAllowedOriginsResult> for ::serde_json::Map<String, ::serde_json::Value> {
        fn from(value: UpdateAllowedOriginsResult) -> Self {
            value.0
        }
    }

    impl From<&UpdateAllowedOriginsResult> for UpdateAllowedOriginsResult {
        fn from(value: &UpdateAllowedOriginsResult) -> Self {
            value.clone()
        }
    }

    impl From<::serde_json::Map<String, ::serde_json::Value>> for UpdateAllowedOriginsResult {
        fn from(value: ::serde_json::Map<String, ::serde_json::Value>) -> Self {
            Self(value)
        }
    }

    ///UpdatePolicyIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "policyId"
    ///  ],
    ///  "properties": {
    ///    "policyCondition": {
    ///      "description": "The condition expression that triggers the Effect
    /// (optional).",
    ///      "type": "string"
    ///    },
    ///    "policyConsensus": {
    ///      "description": "The consensus expression that triggers the Effect
    /// (optional).",
    ///      "type": "string"
    ///    },
    ///    "policyEffect": {
    ///      "$ref": "#/components/schemas/Effect"
    ///    },
    ///    "policyId": {
    ///      "description": "Unique identifier for a given Policy.",
    ///      "type": "string"
    ///    },
    ///    "policyName": {
    ///      "description": "Human-readable name for a Policy.",
    ///      "type": "string"
    ///    },
    ///    "policyNotes": {
    ///      "description": "Accompanying notes for a Policy (optional).",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct UpdatePolicyIntent {
        ///The condition expression that triggers the Effect (optional).
        #[serde(
            rename = "policyCondition",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub policy_condition: Option<String>,
        ///The consensus expression that triggers the Effect (optional).
        #[serde(
            rename = "policyConsensus",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub policy_consensus: Option<String>,
        #[serde(
            rename = "policyEffect",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub policy_effect: Option<Effect>,
        ///Unique identifier for a given Policy.
        #[serde(rename = "policyId")]
        pub policy_id: String,
        ///Human-readable name for a Policy.
        #[serde(
            rename = "policyName",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub policy_name: Option<String>,
        ///Accompanying notes for a Policy (optional).
        #[serde(
            rename = "policyNotes",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub policy_notes: Option<String>,
    }

    impl From<&UpdatePolicyIntent> for UpdatePolicyIntent {
        fn from(value: &UpdatePolicyIntent) -> Self {
            value.clone()
        }
    }

    ///UpdatePolicyRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/UpdatePolicyIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_UPDATE_POLICY"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct UpdatePolicyRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: UpdatePolicyIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: UpdatePolicyRequestType,
    }

    impl From<&UpdatePolicyRequest> for UpdatePolicyRequest {
        fn from(value: &UpdatePolicyRequest) -> Self {
            value.clone()
        }
    }

    ///UpdatePolicyRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_UPDATE_POLICY"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum UpdatePolicyRequestType {
        #[serde(rename = "ACTIVITY_TYPE_UPDATE_POLICY")]
        ActivityTypeUpdatePolicy,
    }

    impl From<&UpdatePolicyRequestType> for UpdatePolicyRequestType {
        fn from(value: &UpdatePolicyRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for UpdatePolicyRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeUpdatePolicy => write!(f, "ACTIVITY_TYPE_UPDATE_POLICY"),
            }
        }
    }

    impl std::str::FromStr for UpdatePolicyRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_UPDATE_POLICY" => Ok(Self::ActivityTypeUpdatePolicy),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for UpdatePolicyRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for UpdatePolicyRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for UpdatePolicyRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///UpdatePolicyResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "policyId"
    ///  ],
    ///  "properties": {
    ///    "policyId": {
    ///      "description": "Unique identifier for a given Policy.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct UpdatePolicyResult {
        ///Unique identifier for a given Policy.
        #[serde(rename = "policyId")]
        pub policy_id: String,
    }

    impl From<&UpdatePolicyResult> for UpdatePolicyResult {
        fn from(value: &UpdatePolicyResult) -> Self {
            value.clone()
        }
    }

    ///UpdatePrivateKeyTagIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "addPrivateKeyIds",
    ///    "privateKeyTagId",
    ///    "removePrivateKeyIds"
    ///  ],
    ///  "properties": {
    ///    "addPrivateKeyIds": {
    ///      "description": "A list of Private Keys IDs to add this tag to.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "newPrivateKeyTagName": {
    ///      "description": "The new, human-readable name for the tag with the
    /// given ID.",
    ///      "type": "string"
    ///    },
    ///    "privateKeyTagId": {
    ///      "description": "Unique identifier for a given Private Key Tag.",
    ///      "type": "string"
    ///    },
    ///    "removePrivateKeyIds": {
    ///      "description": "A list of Private Key IDs to remove this tag
    /// from.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct UpdatePrivateKeyTagIntent {
        ///A list of Private Keys IDs to add this tag to.
        #[serde(rename = "addPrivateKeyIds")]
        pub add_private_key_ids: Vec<String>,
        ///The new, human-readable name for the tag with the given ID.
        #[serde(
            rename = "newPrivateKeyTagName",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub new_private_key_tag_name: Option<String>,
        ///Unique identifier for a given Private Key Tag.
        #[serde(rename = "privateKeyTagId")]
        pub private_key_tag_id: String,
        ///A list of Private Key IDs to remove this tag from.
        #[serde(rename = "removePrivateKeyIds")]
        pub remove_private_key_ids: Vec<String>,
    }

    impl From<&UpdatePrivateKeyTagIntent> for UpdatePrivateKeyTagIntent {
        fn from(value: &UpdatePrivateKeyTagIntent) -> Self {
            value.clone()
        }
    }

    ///UpdatePrivateKeyTagRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/UpdatePrivateKeyTagIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_UPDATE_PRIVATE_KEY_TAG"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct UpdatePrivateKeyTagRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: UpdatePrivateKeyTagIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: UpdatePrivateKeyTagRequestType,
    }

    impl From<&UpdatePrivateKeyTagRequest> for UpdatePrivateKeyTagRequest {
        fn from(value: &UpdatePrivateKeyTagRequest) -> Self {
            value.clone()
        }
    }

    ///UpdatePrivateKeyTagRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_UPDATE_PRIVATE_KEY_TAG"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum UpdatePrivateKeyTagRequestType {
        #[serde(rename = "ACTIVITY_TYPE_UPDATE_PRIVATE_KEY_TAG")]
        ActivityTypeUpdatePrivateKeyTag,
    }

    impl From<&UpdatePrivateKeyTagRequestType> for UpdatePrivateKeyTagRequestType {
        fn from(value: &UpdatePrivateKeyTagRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for UpdatePrivateKeyTagRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeUpdatePrivateKeyTag => {
                    write!(f, "ACTIVITY_TYPE_UPDATE_PRIVATE_KEY_TAG")
                }
            }
        }
    }

    impl std::str::FromStr for UpdatePrivateKeyTagRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_UPDATE_PRIVATE_KEY_TAG" => Ok(Self::ActivityTypeUpdatePrivateKeyTag),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for UpdatePrivateKeyTagRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for UpdatePrivateKeyTagRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for UpdatePrivateKeyTagRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///UpdatePrivateKeyTagResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "privateKeyTagId"
    ///  ],
    ///  "properties": {
    ///    "privateKeyTagId": {
    ///      "description": "Unique identifier for a given Private Key Tag.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct UpdatePrivateKeyTagResult {
        ///Unique identifier for a given Private Key Tag.
        #[serde(rename = "privateKeyTagId")]
        pub private_key_tag_id: String,
    }

    impl From<&UpdatePrivateKeyTagResult> for UpdatePrivateKeyTagResult {
        fn from(value: &UpdatePrivateKeyTagResult) -> Self {
            value.clone()
        }
    }

    ///UpdateRootQuorumIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "threshold",
    ///    "userIds"
    ///  ],
    ///  "properties": {
    ///    "threshold": {
    ///      "description": "The threshold of unique approvals to reach
    /// quorum.",
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "userIds": {
    ///      "description": "The unique identifiers of users who comprise the
    /// quorum set.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct UpdateRootQuorumIntent {
        ///The threshold of unique approvals to reach quorum.
        pub threshold: i32,
        ///The unique identifiers of users who comprise the quorum set.
        #[serde(rename = "userIds")]
        pub user_ids: Vec<String>,
    }

    impl From<&UpdateRootQuorumIntent> for UpdateRootQuorumIntent {
        fn from(value: &UpdateRootQuorumIntent) -> Self {
            value.clone()
        }
    }

    ///UpdateRootQuorumRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/UpdateRootQuorumIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_UPDATE_ROOT_QUORUM"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct UpdateRootQuorumRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: UpdateRootQuorumIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: UpdateRootQuorumRequestType,
    }

    impl From<&UpdateRootQuorumRequest> for UpdateRootQuorumRequest {
        fn from(value: &UpdateRootQuorumRequest) -> Self {
            value.clone()
        }
    }

    ///UpdateRootQuorumRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_UPDATE_ROOT_QUORUM"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum UpdateRootQuorumRequestType {
        #[serde(rename = "ACTIVITY_TYPE_UPDATE_ROOT_QUORUM")]
        ActivityTypeUpdateRootQuorum,
    }

    impl From<&UpdateRootQuorumRequestType> for UpdateRootQuorumRequestType {
        fn from(value: &UpdateRootQuorumRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for UpdateRootQuorumRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeUpdateRootQuorum => write!(f, "ACTIVITY_TYPE_UPDATE_ROOT_QUORUM"),
            }
        }
    }

    impl std::str::FromStr for UpdateRootQuorumRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_UPDATE_ROOT_QUORUM" => Ok(Self::ActivityTypeUpdateRootQuorum),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for UpdateRootQuorumRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for UpdateRootQuorumRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for UpdateRootQuorumRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///UpdateRootQuorumResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object"
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct UpdateRootQuorumResult(pub ::serde_json::Map<String, ::serde_json::Value>);
    impl ::std::ops::Deref for UpdateRootQuorumResult {
        type Target = ::serde_json::Map<String, ::serde_json::Value>;
        fn deref(&self) -> &::serde_json::Map<String, ::serde_json::Value> {
            &self.0
        }
    }

    impl From<UpdateRootQuorumResult> for ::serde_json::Map<String, ::serde_json::Value> {
        fn from(value: UpdateRootQuorumResult) -> Self {
            value.0
        }
    }

    impl From<&UpdateRootQuorumResult> for UpdateRootQuorumResult {
        fn from(value: &UpdateRootQuorumResult) -> Self {
            value.clone()
        }
    }

    impl From<::serde_json::Map<String, ::serde_json::Value>> for UpdateRootQuorumResult {
        fn from(value: ::serde_json::Map<String, ::serde_json::Value>) -> Self {
            Self(value)
        }
    }

    ///UpdateUserIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "userId"
    ///  ],
    ///  "properties": {
    ///    "userEmail": {
    ///      "description": "The user's email address.",
    ///      "type": "string"
    ///    },
    ///    "userId": {
    ///      "description": "Unique identifier for a given User.",
    ///      "type": "string"
    ///    },
    ///    "userName": {
    ///      "description": "Human-readable name for a User.",
    ///      "type": "string"
    ///    },
    ///    "userPhoneNumber": {
    ///      "description": "The user's phone number in E.164 format e.g.
    /// +13214567890",
    ///      "type": "string"
    ///    },
    ///    "userTagIds": {
    ///      "description": "An updated list of User Tags to apply to this
    /// User.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct UpdateUserIntent {
        ///The user's email address.
        #[serde(rename = "userEmail", default, skip_serializing_if = "Option::is_none")]
        pub user_email: Option<String>,
        ///Unique identifier for a given User.
        #[serde(rename = "userId")]
        pub user_id: String,
        ///Human-readable name for a User.
        #[serde(rename = "userName", default, skip_serializing_if = "Option::is_none")]
        pub user_name: Option<String>,
        ///The user's phone number in E.164 format e.g. +13214567890
        #[serde(
            rename = "userPhoneNumber",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub user_phone_number: Option<String>,
        ///An updated list of User Tags to apply to this User.
        #[serde(rename = "userTagIds", default, skip_serializing_if = "Vec::is_empty")]
        pub user_tag_ids: Vec<String>,
    }

    impl From<&UpdateUserIntent> for UpdateUserIntent {
        fn from(value: &UpdateUserIntent) -> Self {
            value.clone()
        }
    }

    ///UpdateUserRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/UpdateUserIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_UPDATE_USER"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct UpdateUserRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: UpdateUserIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: UpdateUserRequestType,
    }

    impl From<&UpdateUserRequest> for UpdateUserRequest {
        fn from(value: &UpdateUserRequest) -> Self {
            value.clone()
        }
    }

    ///UpdateUserRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_UPDATE_USER"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum UpdateUserRequestType {
        #[serde(rename = "ACTIVITY_TYPE_UPDATE_USER")]
        ActivityTypeUpdateUser,
    }

    impl From<&UpdateUserRequestType> for UpdateUserRequestType {
        fn from(value: &UpdateUserRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for UpdateUserRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeUpdateUser => write!(f, "ACTIVITY_TYPE_UPDATE_USER"),
            }
        }
    }

    impl std::str::FromStr for UpdateUserRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_UPDATE_USER" => Ok(Self::ActivityTypeUpdateUser),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for UpdateUserRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for UpdateUserRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for UpdateUserRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///UpdateUserResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "userId"
    ///  ],
    ///  "properties": {
    ///    "userId": {
    ///      "description": "A User ID.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct UpdateUserResult {
        ///A User ID.
        #[serde(rename = "userId")]
        pub user_id: String,
    }

    impl From<&UpdateUserResult> for UpdateUserResult {
        fn from(value: &UpdateUserResult) -> Self {
            value.clone()
        }
    }

    ///UpdateUserTagIntent
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "addUserIds",
    ///    "removeUserIds",
    ///    "userTagId"
    ///  ],
    ///  "properties": {
    ///    "addUserIds": {
    ///      "description": "A list of User IDs to add this tag to.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "newUserTagName": {
    ///      "description": "The new, human-readable name for the tag with the
    /// given ID.",
    ///      "type": "string"
    ///    },
    ///    "removeUserIds": {
    ///      "description": "A list of User IDs to remove this tag from.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "userTagId": {
    ///      "description": "Unique identifier for a given User Tag.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct UpdateUserTagIntent {
        ///A list of User IDs to add this tag to.
        #[serde(rename = "addUserIds")]
        pub add_user_ids: Vec<String>,
        ///The new, human-readable name for the tag with the given ID.
        #[serde(
            rename = "newUserTagName",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub new_user_tag_name: Option<String>,
        ///A list of User IDs to remove this tag from.
        #[serde(rename = "removeUserIds")]
        pub remove_user_ids: Vec<String>,
        ///Unique identifier for a given User Tag.
        #[serde(rename = "userTagId")]
        pub user_tag_id: String,
    }

    impl From<&UpdateUserTagIntent> for UpdateUserTagIntent {
        fn from(value: &UpdateUserTagIntent) -> Self {
            value.clone()
        }
    }

    ///UpdateUserTagRequest
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "organizationId",
    ///    "parameters",
    ///    "timestampMs",
    ///    "type"
    ///  ],
    ///  "properties": {
    ///    "organizationId": {
    ///      "description": "Unique identifier for a given Organization.",
    ///      "type": "string"
    ///    },
    ///    "parameters": {
    ///      "$ref": "#/components/schemas/UpdateUserTagIntent"
    ///    },
    ///    "timestampMs": {
    ///      "description": "Timestamp (in milliseconds) of the request, used to
    /// verify liveness of user requests.",
    ///      "type": "string"
    ///    },
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "ACTIVITY_TYPE_UPDATE_USER_TAG"
    ///      ]
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct UpdateUserTagRequest {
        ///Unique identifier for a given Organization.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        pub parameters: UpdateUserTagIntent,
        ///Timestamp (in milliseconds) of the request, used to verify liveness
        /// of user requests.
        #[serde(rename = "timestampMs")]
        pub timestamp_ms: String,
        #[serde(rename = "type")]
        pub type_: UpdateUserTagRequestType,
    }

    impl From<&UpdateUserTagRequest> for UpdateUserTagRequest {
        fn from(value: &UpdateUserTagRequest) -> Self {
            value.clone()
        }
    }

    ///UpdateUserTagRequestType
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ACTIVITY_TYPE_UPDATE_USER_TAG"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum UpdateUserTagRequestType {
        #[serde(rename = "ACTIVITY_TYPE_UPDATE_USER_TAG")]
        ActivityTypeUpdateUserTag,
    }

    impl From<&UpdateUserTagRequestType> for UpdateUserTagRequestType {
        fn from(value: &UpdateUserTagRequestType) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for UpdateUserTagRequestType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::ActivityTypeUpdateUserTag => write!(f, "ACTIVITY_TYPE_UPDATE_USER_TAG"),
            }
        }
    }

    impl std::str::FromStr for UpdateUserTagRequestType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "ACTIVITY_TYPE_UPDATE_USER_TAG" => Ok(Self::ActivityTypeUpdateUserTag),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for UpdateUserTagRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for UpdateUserTagRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for UpdateUserTagRequestType {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///UpdateUserTagResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "userTagId"
    ///  ],
    ///  "properties": {
    ///    "userTagId": {
    ///      "description": "Unique identifier for a given User Tag.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct UpdateUserTagResult {
        ///Unique identifier for a given User Tag.
        #[serde(rename = "userTagId")]
        pub user_tag_id: String,
    }

    impl From<&UpdateUserTagResult> for UpdateUserTagResult {
        fn from(value: &UpdateUserTagResult) -> Self {
            value.clone()
        }
    }

    ///User
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "apiKeys",
    ///    "authenticators",
    ///    "createdAt",
    ///    "oauthProviders",
    ///    "updatedAt",
    ///    "userId",
    ///    "userName",
    ///    "userTags"
    ///  ],
    ///  "properties": {
    ///    "apiKeys": {
    ///      "description": "A list of API Key parameters.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/ApiKey"
    ///      }
    ///    },
    ///    "authenticators": {
    ///      "description": "A list of Authenticator parameters.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/Authenticator"
    ///      }
    ///    },
    ///    "createdAt": {
    ///      "$ref": "#/components/schemas/external.data.v1.Timestamp"
    ///    },
    ///    "oauthProviders": {
    ///      "description": "A list of Oauth Providers.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/OauthProvider"
    ///      }
    ///    },
    ///    "updatedAt": {
    ///      "$ref": "#/components/schemas/external.data.v1.Timestamp"
    ///    },
    ///    "userEmail": {
    ///      "description": "The user's email address.",
    ///      "type": "string"
    ///    },
    ///    "userId": {
    ///      "description": "Unique identifier for a given User.",
    ///      "type": "string"
    ///    },
    ///    "userName": {
    ///      "description": "Human-readable name for a User.",
    ///      "type": "string"
    ///    },
    ///    "userPhoneNumber": {
    ///      "description": "The user's phone number in E.164 format e.g.
    /// +13214567890",
    ///      "type": "string"
    ///    },
    ///    "userTags": {
    ///      "description": "A list of User Tag IDs.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct User {
        ///A list of API Key parameters.
        #[serde(rename = "apiKeys")]
        pub api_keys: Vec<ApiKey>,
        ///A list of Authenticator parameters.
        pub authenticators: Vec<Authenticator>,
        #[serde(rename = "createdAt")]
        pub created_at: ExternalDataV1Timestamp,
        ///A list of Oauth Providers.
        #[serde(rename = "oauthProviders")]
        pub oauth_providers: Vec<OauthProvider>,
        #[serde(rename = "updatedAt")]
        pub updated_at: ExternalDataV1Timestamp,
        ///The user's email address.
        #[serde(rename = "userEmail", default, skip_serializing_if = "Option::is_none")]
        pub user_email: Option<String>,
        ///Unique identifier for a given User.
        #[serde(rename = "userId")]
        pub user_id: String,
        ///Human-readable name for a User.
        #[serde(rename = "userName")]
        pub user_name: String,
        ///The user's phone number in E.164 format e.g. +13214567890
        #[serde(
            rename = "userPhoneNumber",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub user_phone_number: Option<String>,
        ///A list of User Tag IDs.
        #[serde(rename = "userTags")]
        pub user_tags: Vec<String>,
    }

    impl From<&User> for User {
        fn from(value: &User) -> Self {
            value.clone()
        }
    }

    ///UserParams
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "accessType",
    ///    "apiKeys",
    ///    "authenticators",
    ///    "userName",
    ///    "userTags"
    ///  ],
    ///  "properties": {
    ///    "accessType": {
    ///      "$ref": "#/components/schemas/AccessType"
    ///    },
    ///    "apiKeys": {
    ///      "description": "A list of API Key parameters.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/ApiKeyParams"
    ///      }
    ///    },
    ///    "authenticators": {
    ///      "description": "A list of Authenticator parameters.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/AuthenticatorParams"
    ///      }
    ///    },
    ///    "userEmail": {
    ///      "description": "The user's email address.",
    ///      "type": "string"
    ///    },
    ///    "userName": {
    ///      "description": "Human-readable name for a User.",
    ///      "type": "string"
    ///    },
    ///    "userTags": {
    ///      "description": "A list of User Tag IDs.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct UserParams {
        #[serde(rename = "accessType")]
        pub access_type: AccessType,
        ///A list of API Key parameters.
        #[serde(rename = "apiKeys")]
        pub api_keys: Vec<ApiKeyParams>,
        ///A list of Authenticator parameters.
        pub authenticators: Vec<AuthenticatorParams>,
        ///The user's email address.
        #[serde(rename = "userEmail", default, skip_serializing_if = "Option::is_none")]
        pub user_email: Option<String>,
        ///Human-readable name for a User.
        #[serde(rename = "userName")]
        pub user_name: String,
        ///A list of User Tag IDs.
        #[serde(rename = "userTags")]
        pub user_tags: Vec<String>,
    }

    impl From<&UserParams> for UserParams {
        fn from(value: &UserParams) -> Self {
            value.clone()
        }
    }

    ///UserParamsV2
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "apiKeys",
    ///    "authenticators",
    ///    "userName",
    ///    "userTags"
    ///  ],
    ///  "properties": {
    ///    "apiKeys": {
    ///      "description": "A list of API Key parameters.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/ApiKeyParams"
    ///      }
    ///    },
    ///    "authenticators": {
    ///      "description": "A list of Authenticator parameters.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/AuthenticatorParamsV2"
    ///      }
    ///    },
    ///    "userEmail": {
    ///      "description": "The user's email address.",
    ///      "type": "string"
    ///    },
    ///    "userName": {
    ///      "description": "Human-readable name for a User.",
    ///      "type": "string"
    ///    },
    ///    "userTags": {
    ///      "description": "A list of User Tag IDs.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct UserParamsV2 {
        ///A list of API Key parameters.
        #[serde(rename = "apiKeys")]
        pub api_keys: Vec<ApiKeyParams>,
        ///A list of Authenticator parameters.
        pub authenticators: Vec<AuthenticatorParamsV2>,
        ///The user's email address.
        #[serde(rename = "userEmail", default, skip_serializing_if = "Option::is_none")]
        pub user_email: Option<String>,
        ///Human-readable name for a User.
        #[serde(rename = "userName")]
        pub user_name: String,
        ///A list of User Tag IDs.
        #[serde(rename = "userTags")]
        pub user_tags: Vec<String>,
    }

    impl From<&UserParamsV2> for UserParamsV2 {
        fn from(value: &UserParamsV2) -> Self {
            value.clone()
        }
    }

    ///V1Tag
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "createdAt",
    ///    "tagId",
    ///    "tagName",
    ///    "tagType",
    ///    "updatedAt"
    ///  ],
    ///  "properties": {
    ///    "createdAt": {
    ///      "$ref": "#/components/schemas/external.data.v1.Timestamp"
    ///    },
    ///    "tagId": {
    ///      "description": "Unique identifier for a given Tag.",
    ///      "type": "string"
    ///    },
    ///    "tagName": {
    ///      "description": "Human-readable name for a Tag.",
    ///      "type": "string"
    ///    },
    ///    "tagType": {
    ///      "$ref": "#/components/schemas/TagType"
    ///    },
    ///    "updatedAt": {
    ///      "$ref": "#/components/schemas/external.data.v1.Timestamp"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct V1Tag {
        #[serde(rename = "createdAt")]
        pub created_at: ExternalDataV1Timestamp,
        ///Unique identifier for a given Tag.
        #[serde(rename = "tagId")]
        pub tag_id: String,
        ///Human-readable name for a Tag.
        #[serde(rename = "tagName")]
        pub tag_name: String,
        #[serde(rename = "tagType")]
        pub tag_type: TagType,
        #[serde(rename = "updatedAt")]
        pub updated_at: ExternalDataV1Timestamp,
    }

    impl From<&V1Tag> for V1Tag {
        fn from(value: &V1Tag) -> Self {
            value.clone()
        }
    }

    ///Vote
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "activityId",
    ///    "createdAt",
    ///    "id",
    ///    "message",
    ///    "publicKey",
    ///    "scheme",
    ///    "selection",
    ///    "signature",
    ///    "user",
    ///    "userId"
    ///  ],
    ///  "properties": {
    ///    "activityId": {
    ///      "description": "Unique identifier for a given Activity object.",
    ///      "type": "string"
    ///    },
    ///    "createdAt": {
    ///      "$ref": "#/components/schemas/external.data.v1.Timestamp"
    ///    },
    ///    "id": {
    ///      "description": "Unique identifier for a given Vote object.",
    ///      "type": "string"
    ///    },
    ///    "message": {
    ///      "description": "The raw message being signed within a Vote.",
    ///      "type": "string"
    ///    },
    ///    "publicKey": {
    ///      "description": "The public component of a cryptographic key pair
    /// used to sign messages and transactions.",
    ///      "type": "string"
    ///    },
    ///    "scheme": {
    ///      "description": "Method used to produce a signature.",
    ///      "type": "string"
    ///    },
    ///    "selection": {
    ///      "type": "string",
    ///      "enum": [
    ///        "VOTE_SELECTION_APPROVED",
    ///        "VOTE_SELECTION_REJECTED"
    ///      ]
    ///    },
    ///    "signature": {
    ///      "description": "The signature applied to a particular vote.",
    ///      "type": "string"
    ///    },
    ///    "user": {
    ///      "$ref": "#/components/schemas/User"
    ///    },
    ///    "userId": {
    ///      "description": "Unique identifier for a given User.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct Vote {
        ///Unique identifier for a given Activity object.
        #[serde(rename = "activityId")]
        pub activity_id: String,
        #[serde(rename = "createdAt")]
        pub created_at: ExternalDataV1Timestamp,
        ///Unique identifier for a given Vote object.
        pub id: String,
        ///The raw message being signed within a Vote.
        pub message: String,
        ///The public component of a cryptographic key pair used to sign
        /// messages and transactions.
        #[serde(rename = "publicKey")]
        pub public_key: String,
        ///Method used to produce a signature.
        pub scheme: String,
        pub selection: VoteSelection,
        ///The signature applied to a particular vote.
        pub signature: String,
        pub user: User,
        ///Unique identifier for a given User.
        #[serde(rename = "userId")]
        pub user_id: String,
    }

    impl From<&Vote> for Vote {
        fn from(value: &Vote) -> Self {
            value.clone()
        }
    }

    ///VoteSelection
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "VOTE_SELECTION_APPROVED",
    ///    "VOTE_SELECTION_REJECTED"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        :: serde :: Deserialize,
        :: serde :: Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum VoteSelection {
        #[serde(rename = "VOTE_SELECTION_APPROVED")]
        VoteSelectionApproved,
        #[serde(rename = "VOTE_SELECTION_REJECTED")]
        VoteSelectionRejected,
    }

    impl From<&VoteSelection> for VoteSelection {
        fn from(value: &VoteSelection) -> Self {
            value.clone()
        }
    }

    impl ::std::fmt::Display for VoteSelection {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::VoteSelectionApproved => write!(f, "VOTE_SELECTION_APPROVED"),
                Self::VoteSelectionRejected => write!(f, "VOTE_SELECTION_REJECTED"),
            }
        }
    }

    impl std::str::FromStr for VoteSelection {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> Result<Self, self::error::ConversionError> {
            match value {
                "VOTE_SELECTION_APPROVED" => Ok(Self::VoteSelectionApproved),
                "VOTE_SELECTION_REJECTED" => Ok(Self::VoteSelectionRejected),
                _ => Err("invalid value".into()),
            }
        }
    }

    impl std::convert::TryFrom<&str> for VoteSelection {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<&String> for VoteSelection {
        type Error = self::error::ConversionError;
        fn try_from(value: &String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    impl std::convert::TryFrom<String> for VoteSelection {
        type Error = self::error::ConversionError;
        fn try_from(value: String) -> Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }

    ///Wallet
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "createdAt",
    ///    "exported",
    ///    "imported",
    ///    "updatedAt",
    ///    "walletId",
    ///    "walletName"
    ///  ],
    ///  "properties": {
    ///    "createdAt": {
    ///      "$ref": "#/components/schemas/external.data.v1.Timestamp"
    ///    },
    ///    "exported": {
    ///      "description": "True when a given Wallet is exported, false
    /// otherwise.",
    ///      "type": "boolean"
    ///    },
    ///    "imported": {
    ///      "description": "True when a given Wallet is imported, false
    /// otherwise.",
    ///      "type": "boolean"
    ///    },
    ///    "updatedAt": {
    ///      "$ref": "#/components/schemas/external.data.v1.Timestamp"
    ///    },
    ///    "walletId": {
    ///      "description": "Unique identifier for a given Wallet.",
    ///      "type": "string"
    ///    },
    ///    "walletName": {
    ///      "description": "Human-readable name for a Wallet.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct Wallet {
        #[serde(rename = "createdAt")]
        pub created_at: ExternalDataV1Timestamp,
        ///True when a given Wallet is exported, false otherwise.
        pub exported: bool,
        ///True when a given Wallet is imported, false otherwise.
        pub imported: bool,
        #[serde(rename = "updatedAt")]
        pub updated_at: ExternalDataV1Timestamp,
        ///Unique identifier for a given Wallet.
        #[serde(rename = "walletId")]
        pub wallet_id: String,
        ///Human-readable name for a Wallet.
        #[serde(rename = "walletName")]
        pub wallet_name: String,
    }

    impl From<&Wallet> for Wallet {
        fn from(value: &Wallet) -> Self {
            value.clone()
        }
    }

    ///WalletAccount
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "address",
    ///    "addressFormat",
    ///    "createdAt",
    ///    "curve",
    ///    "organizationId",
    ///    "path",
    ///    "pathFormat",
    ///    "updatedAt",
    ///    "walletId"
    ///  ],
    ///  "properties": {
    ///    "address": {
    ///      "description": "Address generated using the Wallet seed and Account
    /// parameters.",
    ///      "type": "string"
    ///    },
    ///    "addressFormat": {
    ///      "$ref": "#/components/schemas/AddressFormat"
    ///    },
    ///    "createdAt": {
    ///      "$ref": "#/components/schemas/external.data.v1.Timestamp"
    ///    },
    ///    "curve": {
    ///      "$ref": "#/components/schemas/Curve"
    ///    },
    ///    "organizationId": {
    ///      "description": "The Organization the Account belongs to.",
    ///      "type": "string"
    ///    },
    ///    "path": {
    ///      "description": "Path used to generate the Account.",
    ///      "type": "string"
    ///    },
    ///    "pathFormat": {
    ///      "$ref": "#/components/schemas/PathFormat"
    ///    },
    ///    "updatedAt": {
    ///      "$ref": "#/components/schemas/external.data.v1.Timestamp"
    ///    },
    ///    "walletId": {
    ///      "description": "The Wallet the Account was derived from.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct WalletAccount {
        ///Address generated using the Wallet seed and Account parameters.
        pub address: String,
        #[serde(rename = "addressFormat")]
        pub address_format: AddressFormat,
        #[serde(rename = "createdAt")]
        pub created_at: ExternalDataV1Timestamp,
        pub curve: Curve,
        ///The Organization the Account belongs to.
        #[serde(rename = "organizationId")]
        pub organization_id: String,
        ///Path used to generate the Account.
        pub path: String,
        #[serde(rename = "pathFormat")]
        pub path_format: PathFormat,
        #[serde(rename = "updatedAt")]
        pub updated_at: ExternalDataV1Timestamp,
        ///The Wallet the Account was derived from.
        #[serde(rename = "walletId")]
        pub wallet_id: String,
    }

    impl From<&WalletAccount> for WalletAccount {
        fn from(value: &WalletAccount) -> Self {
            value.clone()
        }
    }

    ///WalletAccountParams
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "addressFormat",
    ///    "curve",
    ///    "path",
    ///    "pathFormat"
    ///  ],
    ///  "properties": {
    ///    "addressFormat": {
    ///      "$ref": "#/components/schemas/AddressFormat"
    ///    },
    ///    "curve": {
    ///      "$ref": "#/components/schemas/Curve"
    ///    },
    ///    "path": {
    ///      "description": "Path used to generate a wallet Account.",
    ///      "type": "string"
    ///    },
    ///    "pathFormat": {
    ///      "$ref": "#/components/schemas/PathFormat"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct WalletAccountParams {
        #[serde(rename = "addressFormat")]
        pub address_format: AddressFormat,
        pub curve: Curve,
        ///Path used to generate a wallet Account.
        pub path: String,
        #[serde(rename = "pathFormat")]
        pub path_format: PathFormat,
    }

    impl From<&WalletAccountParams> for WalletAccountParams {
        fn from(value: &WalletAccountParams) -> Self {
            value.clone()
        }
    }

    ///WalletParams
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "accounts",
    ///    "walletName"
    ///  ],
    ///  "properties": {
    ///    "accounts": {
    ///      "description": "A list of wallet Accounts.",
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/components/schemas/WalletAccountParams"
    ///      }
    ///    },
    ///    "mnemonicLength": {
    ///      "description": "Length of mnemonic to generate the Wallet seed.
    /// Defaults to 12. Accepted values: 12, 15, 18, 21, 24.",
    ///      "type": "integer",
    ///      "format": "int32"
    ///    },
    ///    "walletName": {
    ///      "description": "Human-readable name for a Wallet.",
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct WalletParams {
        ///A list of wallet Accounts.
        pub accounts: Vec<WalletAccountParams>,
        ///Length of mnemonic to generate the Wallet seed. Defaults to 12.
        /// Accepted values: 12, 15, 18, 21, 24.
        #[serde(
            rename = "mnemonicLength",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub mnemonic_length: Option<i32>,
        ///Human-readable name for a Wallet.
        #[serde(rename = "walletName")]
        pub wallet_name: String,
    }

    impl From<&WalletParams> for WalletParams {
        fn from(value: &WalletParams) -> Self {
            value.clone()
        }
    }

    ///WalletResult
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "addresses",
    ///    "walletId"
    ///  ],
    ///  "properties": {
    ///    "addresses": {
    ///      "description": "A list of account addresses.",
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "walletId": {
    ///      "type": "string"
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct WalletResult {
        ///A list of account addresses.
        pub addresses: Vec<String>,
        #[serde(rename = "walletId")]
        pub wallet_id: String,
    }

    impl From<&WalletResult> for WalletResult {
        fn from(value: &WalletResult) -> Self {
            value.clone()
        }
    }
}

#[derive(Clone, Debug)]
///Client for API Reference
///
///Review our [API Introduction](../api-introduction) to get started.
///
///Version: 1.0
pub struct Client {
    pub(crate) baseurl: String,
    pub(crate) client: reqwest::Client,
}

impl Client {
    /// Create a new client.
    ///
    /// `baseurl` is the base URL provided to the internal
    /// `reqwest::Client`, and should include a scheme and hostname,
    /// as well as port and a path stem if applicable.
    pub fn new(baseurl: &str) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let client = {
            let dur = std::time::Duration::from_secs(15);
            reqwest::ClientBuilder::new()
                .connect_timeout(dur)
                .timeout(dur)
        };
        #[cfg(target_arch = "wasm32")]
        let client = reqwest::ClientBuilder::new();
        Self::new_with_client(baseurl, client.build().unwrap())
    }

    /// Construct a new client with an existing `reqwest::Client`,
    /// allowing more control over its configuration.
    ///
    /// `baseurl` is the base URL provided to the internal
    /// `reqwest::Client`, and should include a scheme and hostname,
    /// as well as port and a path stem if applicable.
    pub fn new_with_client(baseurl: &str, client: reqwest::Client) -> Self {
        Self {
            baseurl: baseurl.to_string(),
            client,
        }
    }

    /// Get the base URL to which requests are made.
    pub fn baseurl(&self) -> &String {
        &self.baseurl
    }

    /// Get the internal `reqwest::Client` used to make requests.
    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    /// Get the version of this API.
    ///
    /// This string is pulled directly from the source OpenAPI
    /// document and may be in any format the API selects.
    pub fn api_version(&self) -> &'static str {
        "1.0"
    }
}

#[allow(clippy::all)]
impl Client {
    ///Get Activity
    ///
    ///Get details about an Activity
    ///
    ///Sends a `POST` request to `/public/v1/query/get_activity`
    pub async fn get_activity<'a>(
        &'a self,
        body: &'a types::GetActivityRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/query/get_activity", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Get API key
    ///
    ///Get details about an API key
    ///
    ///Sends a `POST` request to `/public/v1/query/get_api_key`
    pub async fn get_api_key<'a>(
        &'a self,
        body: &'a types::GetApiKeyRequest,
    ) -> Result<ResponseValue<types::GetApiKeyResponse>, Error<()>> {
        let url = format!("{}/public/v1/query/get_api_key", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Get API key
    ///
    ///Get details about API keys for a user
    ///
    ///Sends a `POST` request to `/public/v1/query/get_api_keys`
    pub async fn get_api_keys<'a>(
        &'a self,
        body: &'a types::GetApiKeysRequest,
    ) -> Result<ResponseValue<types::GetApiKeysResponse>, Error<()>> {
        let url = format!("{}/public/v1/query/get_api_keys", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Get Authenticator
    ///
    ///Get details about an authenticator
    ///
    ///Sends a `POST` request to `/public/v1/query/get_authenticator`
    pub async fn get_authenticator<'a>(
        &'a self,
        body: &'a types::GetAuthenticatorRequest,
    ) -> Result<ResponseValue<types::GetAuthenticatorResponse>, Error<()>> {
        let url = format!("{}/public/v1/query/get_authenticator", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Get Authenticators
    ///
    ///Get details about authenticators for a user
    ///
    ///Sends a `POST` request to `/public/v1/query/get_authenticators`
    pub async fn get_authenticators<'a>(
        &'a self,
        body: &'a types::GetAuthenticatorsRequest,
    ) -> Result<ResponseValue<types::GetAuthenticatorsResponse>, Error<()>> {
        let url = format!("{}/public/v1/query/get_authenticators", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Get Oauth providers
    ///
    ///Get details about Oauth providers for a user
    ///
    ///Sends a `POST` request to `/public/v1/query/get_oauth_providers`
    pub async fn get_oauth_providers<'a>(
        &'a self,
        body: &'a types::GetOauthProvidersRequest,
    ) -> Result<ResponseValue<types::GetOauthProvidersResponse>, Error<()>> {
        let url = format!("{}/public/v1/query/get_oauth_providers", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Get Configs
    ///
    ///Get quorum settings and features for an organization
    ///
    ///Sends a `POST` request to `/public/v1/query/get_organization_configs`
    pub async fn get_organization_configs<'a>(
        &'a self,
        body: &'a types::GetOrganizationConfigsRequest,
    ) -> Result<ResponseValue<types::GetOrganizationConfigsResponse>, Error<()>> {
        let url = format!("{}/public/v1/query/get_organization_configs", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Get Policy
    ///
    ///Get details about a Policy
    ///
    ///Sends a `POST` request to `/public/v1/query/get_policy`
    pub async fn get_policy<'a>(
        &'a self,
        body: &'a types::GetPolicyRequest,
    ) -> Result<ResponseValue<types::GetPolicyResponse>, Error<()>> {
        let url = format!("{}/public/v1/query/get_policy", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Get Private Key
    ///
    ///Get details about a Private Key
    ///
    ///Sends a `POST` request to `/public/v1/query/get_private_key`
    pub async fn get_private_key<'a>(
        &'a self,
        body: &'a types::GetPrivateKeyRequest,
    ) -> Result<ResponseValue<types::GetPrivateKeyResponse>, Error<()>> {
        let url = format!("{}/public/v1/query/get_private_key", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Get User
    ///
    ///Get details about a User
    ///
    ///Sends a `POST` request to `/public/v1/query/get_user`
    pub async fn get_user<'a>(
        &'a self,
        body: &'a types::GetUserRequest,
    ) -> Result<ResponseValue<types::GetUserResponse>, Error<()>> {
        let url = format!("{}/public/v1/query/get_user", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Get Wallet
    ///
    ///Get details about a Wallet
    ///
    ///Sends a `POST` request to `/public/v1/query/get_wallet`
    pub async fn get_wallet<'a>(
        &'a self,
        body: &'a types::GetWalletRequest,
    ) -> Result<ResponseValue<types::GetWalletResponse>, Error<()>> {
        let url = format!("{}/public/v1/query/get_wallet", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///List Activities
    ///
    ///List all Activities within an Organization
    ///
    ///Sends a `POST` request to `/public/v1/query/list_activities`
    pub async fn get_activities<'a>(
        &'a self,
        body: &'a types::GetActivitiesRequest,
    ) -> Result<ResponseValue<types::GetActivitiesResponse>, Error<()>> {
        let url = format!("{}/public/v1/query/list_activities", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///List Policies
    ///
    ///List all Policies within an Organization
    ///
    ///Sends a `POST` request to `/public/v1/query/list_policies`
    pub async fn get_policies<'a>(
        &'a self,
        body: &'a types::GetPoliciesRequest,
    ) -> Result<ResponseValue<types::GetPoliciesResponse>, Error<()>> {
        let url = format!("{}/public/v1/query/list_policies", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///List Private Key Tags
    ///
    ///List all Private Key Tags within an Organization
    ///
    ///Sends a `POST` request to `/public/v1/query/list_private_key_tags`
    pub async fn list_private_key_tags<'a>(
        &'a self,
        body: &'a types::ListPrivateKeyTagsRequest,
    ) -> Result<ResponseValue<types::ListPrivateKeyTagsResponse>, Error<()>> {
        let url = format!("{}/public/v1/query/list_private_key_tags", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///List Private Keys
    ///
    ///List all Private Keys within an Organization
    ///
    ///Sends a `POST` request to `/public/v1/query/list_private_keys`
    pub async fn get_private_keys<'a>(
        &'a self,
        body: &'a types::GetPrivateKeysRequest,
    ) -> Result<ResponseValue<types::GetPrivateKeysResponse>, Error<()>> {
        let url = format!("{}/public/v1/query/list_private_keys", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Get Suborgs
    ///
    ///Get all suborg IDs associated given a parent org ID and an optional
    /// filter.
    ///
    ///Sends a `POST` request to `/public/v1/query/list_suborgs`
    pub async fn get_sub_org_ids<'a>(
        &'a self,
        body: &'a types::GetSubOrgIdsRequest,
    ) -> Result<ResponseValue<types::GetSubOrgIdsResponse>, Error<()>> {
        let url = format!("{}/public/v1/query/list_suborgs", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///List User Tags
    ///
    ///List all User Tags within an Organization
    ///
    ///Sends a `POST` request to `/public/v1/query/list_user_tags`
    pub async fn list_user_tags<'a>(
        &'a self,
        body: &'a types::ListUserTagsRequest,
    ) -> Result<ResponseValue<types::ListUserTagsResponse>, Error<()>> {
        let url = format!("{}/public/v1/query/list_user_tags", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///List Users
    ///
    ///List all Users within an Organization
    ///
    ///Sends a `POST` request to `/public/v1/query/list_users`
    pub async fn get_users<'a>(
        &'a self,
        body: &'a types::GetUsersRequest,
    ) -> Result<ResponseValue<types::GetUsersResponse>, Error<()>> {
        let url = format!("{}/public/v1/query/list_users", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///List Wallets Accounts
    ///
    ///List all Accounts wirhin a Wallet
    ///
    ///Sends a `POST` request to `/public/v1/query/list_wallet_accounts`
    pub async fn get_wallet_accounts<'a>(
        &'a self,
        body: &'a types::GetWalletAccountsRequest,
    ) -> Result<ResponseValue<types::GetWalletAccountsResponse>, Error<()>> {
        let url = format!("{}/public/v1/query/list_wallet_accounts", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///List Wallets
    ///
    ///List all Wallets within an Organization
    ///
    ///Sends a `POST` request to `/public/v1/query/list_wallets`
    pub async fn get_wallets<'a>(
        &'a self,
        body: &'a types::GetWalletsRequest,
    ) -> Result<ResponseValue<types::GetWalletsResponse>, Error<()>> {
        let url = format!("{}/public/v1/query/list_wallets", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Who am I?
    ///
    ///Get basic information about your current API or WebAuthN user and their
    /// organization. Affords Sub-Organization look ups via Parent Organization
    /// for WebAuthN or API key users.
    ///
    ///Sends a `POST` request to `/public/v1/query/whoami`
    pub async fn get_whoami<'a>(
        &'a self,
        body: &'a types::GetWhoamiRequest,
    ) -> Result<ResponseValue<types::GetWhoamiResponse>, Error<()>> {
        let url = format!("{}/public/v1/query/whoami", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Approve Activity
    ///
    ///Approve an Activity
    ///
    ///Sends a `POST` request to `/public/v1/submit/approve_activity`
    pub async fn approve_activity<'a>(
        &'a self,
        body: &'a types::ApproveActivityRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/approve_activity", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Create API Keys
    ///
    ///Add api keys to an existing User
    ///
    ///Sends a `POST` request to `/public/v1/submit/create_api_keys`
    pub async fn create_api_keys<'a>(
        &'a self,
        body: &'a types::CreateApiKeysRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/create_api_keys", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Create Authenticators
    ///
    ///Create Authenticators to authenticate requests to Turnkey
    ///
    ///Sends a `POST` request to `/public/v1/submit/create_authenticators`
    pub async fn create_authenticators<'a>(
        &'a self,
        body: &'a types::CreateAuthenticatorsRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/create_authenticators", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Create Invitations
    ///
    ///Create Invitations to join an existing Organization
    ///
    ///Sends a `POST` request to `/public/v1/submit/create_invitations`
    pub async fn create_invitations<'a>(
        &'a self,
        body: &'a types::CreateInvitationsRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/create_invitations", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Create Oauth Providers
    ///
    ///Creates Oauth providers for a specified user - BETA
    ///
    ///Sends a `POST` request to `/public/v1/submit/create_oauth_providers`
    pub async fn create_oauth_providers<'a>(
        &'a self,
        body: &'a types::CreateOauthProvidersRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/create_oauth_providers", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Create Policies
    ///
    ///Create new Policies
    ///
    ///Sends a `POST` request to `/public/v1/submit/create_policies`
    pub async fn create_policies<'a>(
        &'a self,
        body: &'a types::CreatePoliciesRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/create_policies", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Create Policy
    ///
    ///Create a new Policy
    ///
    ///Sends a `POST` request to `/public/v1/submit/create_policy`
    pub async fn create_policy<'a>(
        &'a self,
        body: &'a types::CreatePolicyRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/create_policy", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Create Private Key Tag
    ///
    ///Create a private key tag and add it to private keys.
    ///
    ///Sends a `POST` request to `/public/v1/submit/create_private_key_tag`
    pub async fn create_private_key_tag<'a>(
        &'a self,
        body: &'a types::CreatePrivateKeyTagRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/create_private_key_tag", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Create Private Keys
    ///
    ///Create new Private Keys
    ///
    ///Sends a `POST` request to `/public/v1/submit/create_private_keys`
    pub async fn create_private_keys<'a>(
        &'a self,
        body: &'a types::CreatePrivateKeysRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/create_private_keys", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Create Read Only Session
    ///
    ///Create a read only session for a user (valid for 1 hour)
    ///
    ///Sends a `POST` request to `/public/v1/submit/create_read_only_session`
    pub async fn create_read_only_session<'a>(
        &'a self,
        body: &'a types::CreateReadOnlySessionRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/create_read_only_session", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Create Read Write Session
    ///
    ///Create a read write session for a user
    ///
    ///Sends a `POST` request to `/public/v1/submit/create_read_write_session`
    pub async fn create_read_write_session<'a>(
        &'a self,
        body: &'a types::CreateReadWriteSessionRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!(
            "{}/public/v1/submit/create_read_write_session",
            self.baseurl,
        );
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Create Sub-Organization
    ///
    ///Create a new Sub-Organization
    ///
    ///Sends a `POST` request to `/public/v1/submit/create_sub_organization`
    pub async fn create_sub_organization<'a>(
        &'a self,
        body: &'a types::CreateSubOrganizationRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/create_sub_organization", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Create User Tag
    ///
    ///Create a user tag and add it to users.
    ///
    ///Sends a `POST` request to `/public/v1/submit/create_user_tag`
    pub async fn create_user_tag<'a>(
        &'a self,
        body: &'a types::CreateUserTagRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/create_user_tag", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Create Users
    ///
    ///Create Users in an existing Organization
    ///
    ///Sends a `POST` request to `/public/v1/submit/create_users`
    pub async fn create_users<'a>(
        &'a self,
        body: &'a types::CreateUsersRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/create_users", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Create Wallet
    ///
    ///Create a Wallet and derive addresses
    ///
    ///Sends a `POST` request to `/public/v1/submit/create_wallet`
    pub async fn create_wallet<'a>(
        &'a self,
        body: &'a types::CreateWalletRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/create_wallet", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Create Wallet Accounts
    ///
    ///Derive additional addresses using an existing wallet
    ///
    ///Sends a `POST` request to `/public/v1/submit/create_wallet_accounts`
    pub async fn create_wallet_accounts<'a>(
        &'a self,
        body: &'a types::CreateWalletAccountsRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/create_wallet_accounts", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Delete API Keys
    ///
    ///Remove api keys from a User
    ///
    ///Sends a `POST` request to `/public/v1/submit/delete_api_keys`
    pub async fn delete_api_keys<'a>(
        &'a self,
        body: &'a types::DeleteApiKeysRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/delete_api_keys", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Delete Authenticators
    ///
    ///Remove authenticators from a User
    ///
    ///Sends a `POST` request to `/public/v1/submit/delete_authenticators`
    pub async fn delete_authenticators<'a>(
        &'a self,
        body: &'a types::DeleteAuthenticatorsRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/delete_authenticators", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Delete Invitation
    ///
    ///Delete an existing Invitation
    ///
    ///Sends a `POST` request to `/public/v1/submit/delete_invitation`
    pub async fn delete_invitation<'a>(
        &'a self,
        body: &'a types::DeleteInvitationRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/delete_invitation", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Delete Oauth Providers
    ///
    ///Removes Oauth providers for a specified user - BETA
    ///
    ///Sends a `POST` request to `/public/v1/submit/delete_oauth_providers`
    pub async fn delete_oauth_providers<'a>(
        &'a self,
        body: &'a types::DeleteOauthProvidersRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/delete_oauth_providers", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Delete Policy
    ///
    ///Delete an existing Policy
    ///
    ///Sends a `POST` request to `/public/v1/submit/delete_policy`
    pub async fn delete_policy<'a>(
        &'a self,
        body: &'a types::DeletePolicyRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/delete_policy", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Delete Private Key Tags
    ///
    ///Delete Private Key Tags within an Organization
    ///
    ///Sends a `POST` request to `/public/v1/submit/delete_private_key_tags`
    pub async fn delete_private_key_tags<'a>(
        &'a self,
        body: &'a types::DeletePrivateKeyTagsRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/delete_private_key_tags", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Delete Private Keys
    ///
    ///Deletes private keys for an organization
    ///
    ///Sends a `POST` request to `/public/v1/submit/delete_private_keys`
    pub async fn delete_private_keys<'a>(
        &'a self,
        body: &'a types::DeletePrivateKeysRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/delete_private_keys", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Delete Sub Organization
    ///
    ///Deletes a sub organization
    ///
    ///Sends a `POST` request to `/public/v1/submit/delete_sub_organization`
    pub async fn delete_sub_organization<'a>(
        &'a self,
        body: &'a types::DeleteSubOrganizationRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/delete_sub_organization", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Delete User Tags
    ///
    ///Delete User Tags within an Organization
    ///
    ///Sends a `POST` request to `/public/v1/submit/delete_user_tags`
    pub async fn delete_user_tags<'a>(
        &'a self,
        body: &'a types::DeleteUserTagsRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/delete_user_tags", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Delete Users
    ///
    ///Delete Users within an Organization
    ///
    ///Sends a `POST` request to `/public/v1/submit/delete_users`
    pub async fn delete_users<'a>(
        &'a self,
        body: &'a types::DeleteUsersRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/delete_users", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Delete Wallets
    ///
    ///Deletes wallets for an organization
    ///
    ///Sends a `POST` request to `/public/v1/submit/delete_wallets`
    pub async fn delete_wallets<'a>(
        &'a self,
        body: &'a types::DeleteWalletsRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/delete_wallets", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Perform Email Auth
    ///
    ///Authenticate a user via Email
    ///
    ///Sends a `POST` request to `/public/v1/submit/email_auth`
    pub async fn email_auth<'a>(
        &'a self,
        body: &'a types::EmailAuthRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/email_auth", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Export Private Key
    ///
    ///Exports a Private Key
    ///
    ///Sends a `POST` request to `/public/v1/submit/export_private_key`
    pub async fn export_private_key<'a>(
        &'a self,
        body: &'a types::ExportPrivateKeyRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/export_private_key", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Export Wallet
    ///
    ///Exports a Wallet
    ///
    ///Sends a `POST` request to `/public/v1/submit/export_wallet`
    pub async fn export_wallet<'a>(
        &'a self,
        body: &'a types::ExportWalletRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/export_wallet", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Export Wallet Account
    ///
    ///Exports a Wallet Account
    ///
    ///Sends a `POST` request to `/public/v1/submit/export_wallet_account`
    pub async fn export_wallet_account<'a>(
        &'a self,
        body: &'a types::ExportWalletAccountRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/export_wallet_account", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Import Private Key
    ///
    ///Imports a private key
    ///
    ///Sends a `POST` request to `/public/v1/submit/import_private_key`
    pub async fn import_private_key<'a>(
        &'a self,
        body: &'a types::ImportPrivateKeyRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/import_private_key", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Import Wallet
    ///
    ///Imports a wallet
    ///
    ///Sends a `POST` request to `/public/v1/submit/import_wallet`
    pub async fn import_wallet<'a>(
        &'a self,
        body: &'a types::ImportWalletRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/import_wallet", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Init Import Private Key
    ///
    ///Initializes a new private key import
    ///
    ///Sends a `POST` request to `/public/v1/submit/init_import_private_key`
    pub async fn init_import_private_key<'a>(
        &'a self,
        body: &'a types::InitImportPrivateKeyRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/init_import_private_key", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Init Import Wallet
    ///
    ///Initializes a new wallet import
    ///
    ///Sends a `POST` request to `/public/v1/submit/init_import_wallet`
    pub async fn init_import_wallet<'a>(
        &'a self,
        body: &'a types::InitImportWalletRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/init_import_wallet", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Init OTP auth
    ///
    ///Initiate an OTP auth activity
    ///
    ///Sends a `POST` request to `/public/v1/submit/init_otp_auth`
    pub async fn init_otp_auth<'a>(
        &'a self,
        body: &'a types::InitOtpAuthRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/init_otp_auth", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Init Email Recovery
    ///
    ///Initializes a new email recovery
    ///
    ///Sends a `POST` request to `/public/v1/submit/init_user_email_recovery`
    pub async fn init_user_email_recovery<'a>(
        &'a self,
        body: &'a types::InitUserEmailRecoveryRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/init_user_email_recovery", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Oauth
    ///
    ///Authenticate a user with an Oidc token (Oauth) - BETA
    ///
    ///Sends a `POST` request to `/public/v1/submit/oauth`
    pub async fn oauth<'a>(
        &'a self,
        body: &'a types::OauthRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/oauth", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///OTP auth
    ///
    ///Authenticate a user with an OTP code sent via email or SMS
    ///
    ///Sends a `POST` request to `/public/v1/submit/otp_auth`
    pub async fn otp_auth<'a>(
        &'a self,
        body: &'a types::OtpAuthRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/otp_auth", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Recover a user
    ///
    ///Completes the process of recovering a user by adding an authenticator
    ///
    ///Sends a `POST` request to `/public/v1/submit/recover_user`
    pub async fn recover_user<'a>(
        &'a self,
        body: &'a types::RecoverUserRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/recover_user", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Reject Activity
    ///
    ///Reject an Activity
    ///
    ///Sends a `POST` request to `/public/v1/submit/reject_activity`
    pub async fn reject_activity<'a>(
        &'a self,
        body: &'a types::RejectActivityRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/reject_activity", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Remove Organization Feature
    ///
    ///Removes an organization feature. This activity must be approved by the
    /// current root quorum.
    ///
    ///Sends a `POST` request to
    /// `/public/v1/submit/remove_organization_feature`
    pub async fn remove_organization_feature<'a>(
        &'a self,
        body: &'a types::RemoveOrganizationFeatureRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!(
            "{}/public/v1/submit/remove_organization_feature",
            self.baseurl,
        );
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Set Organization Feature
    ///
    ///Sets an organization feature. This activity must be approved by the
    /// current root quorum.
    ///
    ///Sends a `POST` request to `/public/v1/submit/set_organization_feature`
    pub async fn set_organization_feature<'a>(
        &'a self,
        body: &'a types::SetOrganizationFeatureRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/set_organization_feature", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Sign Raw Payload
    ///
    ///Sign a raw payload
    ///
    ///Sends a `POST` request to `/public/v1/submit/sign_raw_payload`
    pub async fn sign_raw_payload<'a>(
        &'a self,
        body: &'a types::SignRawPayloadRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/sign_raw_payload", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Sign Raw Payloads
    ///
    ///Sign multiple raw payloads with the same signing parameters
    ///
    ///Sends a `POST` request to `/public/v1/submit/sign_raw_payloads`
    pub async fn sign_raw_payloads<'a>(
        &'a self,
        body: &'a types::SignRawPayloadsRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/sign_raw_payloads", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Sign Transaction
    ///
    ///Sign a transaction
    ///
    ///Sends a `POST` request to `/public/v1/submit/sign_transaction`
    pub async fn sign_transaction<'a>(
        &'a self,
        body: &'a types::SignTransactionRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/sign_transaction", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Update Policy
    ///
    ///Update an existing Policy
    ///
    ///Sends a `POST` request to `/public/v1/submit/update_policy`
    pub async fn update_policy<'a>(
        &'a self,
        body: &'a types::UpdatePolicyRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/update_policy", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Update Private Key Tag
    ///
    ///Update human-readable name or associated private keys. Note that this
    /// activity is atomic: all of the updates will succeed at once, or all of
    /// them will fail.
    ///
    ///Sends a `POST` request to `/public/v1/submit/update_private_key_tag`
    pub async fn update_private_key_tag<'a>(
        &'a self,
        body: &'a types::UpdatePrivateKeyTagRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/update_private_key_tag", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Update Root Quorum
    ///
    ///Set the threshold and members of the root quorum. This activity must be
    /// approved by the current root quorum.
    ///
    ///Sends a `POST` request to `/public/v1/submit/update_root_quorum`
    pub async fn update_root_quorum<'a>(
        &'a self,
        body: &'a types::UpdateRootQuorumRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/update_root_quorum", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Update User
    ///
    ///Update a User in an existing Organization
    ///
    ///Sends a `POST` request to `/public/v1/submit/update_user`
    pub async fn update_user<'a>(
        &'a self,
        body: &'a types::UpdateUserRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/update_user", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    ///Update User Tag
    ///
    ///Update human-readable name or associated users. Note that this activity
    /// is atomic: all of the updates will succeed at once, or all of them will
    /// fail.
    ///
    ///Sends a `POST` request to `/public/v1/submit/update_user_tag`
    pub async fn update_user_tag<'a>(
        &'a self,
        body: &'a types::UpdateUserTagRequest,
    ) -> Result<ResponseValue<types::ActivityResponse>, Error<()>> {
        let url = format!("{}/public/v1/submit/update_user_tag", self.baseurl,);
        #[allow(unused_mut)]
        let mut request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
}

/// Items consumers will typically use such as the Client.
pub mod prelude {
    #[allow(unused_imports)]
    pub use super::Client;
}
