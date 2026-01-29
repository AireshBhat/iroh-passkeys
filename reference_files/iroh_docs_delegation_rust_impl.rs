// iroh-docs Capability Delegation - Rust Implementation
// Identity Key ↔ Transport Key Separation for Local-First Identity Wallet
//
// This file contains production-ready Rust code examples for implementing
// the capability delegation system described in the design document.

use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use serde::{Serialize, Deserialize};
use blake3::Hash;
use cid::Cid;

// ============================================================================
// SECTION 1: Core Types and Structures
// ============================================================================

/// Represents a Decentralized Identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Did(pub String);

impl Did {
    pub fn new(did: impl Into<String>) -> Self {
        Self(did.into())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// UCAN Capability structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    /// Resource URI (e.g., "iroh:namespace:...")
    pub with_resource: String,
    /// Ability (e.g., "iroh/docs/write")
    pub can: String,
    /// Capability constraints (attenuation)
    #[serde(skip_serializing_if = "Option::isNone")]
    pub nb: Option<serde_json::Value>,
}

/// UCAN Token structure (simplified)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ucan {
    /// UCAN version
    pub ucv: String,
    /// Issuer DID
    pub iss: Did,
    /// Audience DID
    pub aud: Did,
    /// Subject DID
    pub sub: Did,
    /// Not before (Unix timestamp)
    #[serde(skip_serializing_if = "Option::isNone")]
    pub nbf: Option<u64>,
    /// Expiration (Unix timestamp)
    #[serde(skip_serializing_if = "Option::isNone")]
    pub exp: Option<u64>,
    /// Nonce for uniqueness
    pub nonce: Vec<u8>,
    /// Capabilities
    pub att: Vec<Capability>,
    /// Proof chain (parent UCAN CIDs)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub prf: Vec<Cid>,
    /// Metadata
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub meta: HashMap<String, serde_json::Value>,
    /// Signature (base64-encoded)
    #[serde(skip_serializing_if = "Option::isNone")]
    pub sig: Option<String>,
}

impl Ucan {
    /// Convert UCAN to CID for referencing
    pub fn to_cid(&self) -> Cid {
        let bytes = serde_json::to_vec(self).unwrap();
        let hash = blake3::hash(&bytes);
        Cid::new_v1(0x55, cid::multihash::Multihash::from_bytes(&hash.as_bytes()[..34]).unwrap())
    }
    
    /// Verify UCAN signature
    pub fn verify(&self, verifying_key: &VerifyingKey) -> Result<()> {
        let sig = self.sig.as_ref()
            .ok_or_else(|| anyhow!("UCAN not signed"))?;
        let sig_bytes = base64_decode(sig)?;
        let signature = Signature::from_bytes(&sig_bytes.try_into()
            .map_err(|_| anyhow!("Invalid signature length"))?);
        
        // Create unsigned copy for verification
        let mut unsigned = self.clone();
        unsigned.sig = None;
        let message = serde_json::to_vec(&unsigned)?;
        
        verifying_key.verify(&message, &signature)
            .map_err(|e| anyhow!("Signature verification failed: {}", e))
    }
    
    pub fn issuer(&self) -> &Did {
        &self.iss
    }
    
    pub fn audience(&self) -> &Did {
        &self.aud
    }
}

/// iroh-docs Namespace Secret (write capability)
#[derive(Debug, Clone)]
pub struct NamespaceSecret {
    signing_key: SigningKey,
}

impl NamespaceSecret {
    pub fn new<R: rand::CryptoRng + rand::RngCore>(rng: &mut R) -> Self {
        Self {
            signing_key: SigningKey::generate(rng),
        }
    }
    
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self {
            signing_key: SigningKey::from_bytes(bytes),
        }
    }
    
    pub fn to_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }
    
    pub fn id(&self) -> NamespaceId {
        NamespaceId(self.signing_key.verifying_key().to_bytes())
    }
    
    pub fn public_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }
    
    pub fn sign(&self, message: &[u8]) -> Signature {
        self.signing_key.sign(message)
    }
}

/// iroh-docs Namespace ID (public identifier)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NamespaceId(pub [u8; 32]);

impl std::fmt::Display for NamespaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", base58_encode(&self.0))
    }
}

/// iroh-docs Author (entry signing key)
#[derive(Debug, Clone)]
pub struct Author {
    signing_key: SigningKey,
}

impl Author {
    pub fn new<R: rand::CryptoRng + rand::RngCore>(rng: &mut R) -> Self {
        Self {
            signing_key: SigningKey::generate(rng),
        }
    }
    
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self {
            signing_key: SigningKey::from_bytes(bytes),
        }
    }
    
    pub fn from_signing_key(key: SigningKey) -> Self {
        Self { signing_key: key }
    }
    
    pub fn id(&self) -> AuthorId {
        AuthorId(self.signing_key.verifying_key().to_bytes())
    }
    
    pub fn public_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }
    
    pub fn sign(&self, message: &[u8]) -> Signature {
        self.signing_key.sign(message)
    }
    
    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<()> {
        self.public_key().verify(message, signature)
            .map_err(|e| anyhow!("Signature verification failed: {}", e))
    }
    
    pub fn to_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }
}

impl From<SigningKey> for Author {
    fn from(key: SigningKey) -> Self {
        Self::from_signing_key(key)
    }
}

/// iroh-docs Author ID (public identifier)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AuthorId(pub [u8; 32]);

impl std::fmt::Display for AuthorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", base58_encode(&self.0))
    }
}

// ============================================================================
// SECTION 2: Delegation Bundle
// ============================================================================

/// Complete delegation bundle from Identity Key to Transport Key
#[derive(Debug, Clone)]
pub struct DelegationBundle {
    /// UCAN proving authority
    pub ucan: Ucan,
    /// Namespace secret (write capability)
    pub namespace_secret: NamespaceSecret,
    /// Author for signing entries
    pub author: Author,
    /// Identity DID
    pub identity_did: Did,
}

/// Imported and verified delegation
#[derive(Debug, Clone)]
pub struct ImportedDelegation {
    pub ucan: Ucan,
    pub namespace: NamespaceSecret,
    pub author: Author,
    pub identity_did: Did,
    pub imported_at: u64,
}

// ============================================================================
// SECTION 3: UCAN Builder
// ============================================================================

/// Builder for creating UCAN delegation tokens
pub struct UcanBuilder {
    issuer: Option<Did>,
    audience: Option<Did>,
    subject: Option<Did>,
    not_before: Option<u64>,
    expires_at: Option<u64>,
    capabilities: Vec<Capability>,
    nonce: Option<Vec<u8>>,
    meta: HashMap<String, serde_json::Value>,
    proofs: Vec<Cid>,
}

impl Default for UcanBuilder {
    fn default() -> Self {
        Self {
            issuer: None,
            audience: None,
            subject: None,
            not_before: None,
            expires_at: None,
            capabilities: Vec::new(),
            nonce: None,
            meta: HashMap::new(),
            proofs: Vec::new(),
        }
    }
}

impl UcanBuilder {
    pub fn issued_by(mut self, did: &Did) -> Self {
        self.issuer = Some(did.clone());
        self
    }
    
    pub fn for_audience(mut self, did: &Did) -> Self {
        self.audience = Some(did.clone());
        self
    }
    
    pub fn with_subject(mut self, did: &Did) -> Self {
        self.subject = Some(did.clone());
        self
    }
    
    pub fn not_before(mut self, timestamp: u64) -> Self {
        self.not_before = Some(timestamp);
        self
    }
    
    pub fn expires_at(mut self, timestamp: u64) -> Self {
        self.expires_at = Some(timestamp);
        self
    }
    
    pub fn with_capability(mut self, cap: Capability) -> Self {
        self.capabilities.push(cap);
        self
    }
    
    pub fn with_capabilities(mut self, caps: Vec<Capability>) -> Self {
        self.capabilities.extend(caps);
        self
    }
    
    pub fn with_nonce(mut self, nonce: Vec<u8>) -> Self {
        self.nonce = Some(nonce);
        self
    }
    
    pub fn with_meta(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        let value = serde_json::to_value(value).unwrap_or(serde_json::Value::Null);
        self.meta.insert(key.into(), value);
        self
    }
    
    pub fn with_proof(mut self, cid: Cid) -> Self {
        self.proofs.push(cid);
        self
    }
    
    pub fn sign(self, signing_key: &SigningKey) -> Result<Ucan> {
        let issuer = self.issuer.ok_or_else(|| anyhow!("Issuer required"))?;
        let audience = self.audience.ok_or_else(|| anyhow!("Audience required"))?;
        let subject = self.subject.ok_or_else(|| anyhow!("Subject required"))?;
        
        let mut ucan = Ucan {
            ucv: "1.0.0-rc.1".to_string(),
            iss: issuer,
            aud: audience,
            sub: subject,
            nbf: self.not_before,
            exp: self.expires_at,
            nonce: self.nonce.unwrap_or_else(generate_nonce),
            att: self.capabilities,
            prf: self.proofs,
            meta: self.meta,
            sig: None,
        };
        
        // Sign the UCAN
        let message = serde_json::to_vec(&ucan)?;
        let signature = signing_key.sign(&message);
        ucan.sig = Some(base64_encode(signature.to_bytes().as_slice()));
        
        Ok(ucan)
    }
}

// ============================================================================
// SECTION 4: Delegation Functions
// ============================================================================

/// Create a delegation UCAN from Identity Key to Transport Key
pub fn create_delegation_ucan(
    identity_key: &SigningKey,
    transport_key: &SigningKey,
    namespace: &NamespaceSecret,
    validity_hours: u64,
) -> Result<Ucan> {
    let identity_did = did_from_signing_key(identity_key);
    let transport_did = did_from_signing_key(transport_key);
    let namespace_id = namespace.id();
    
    let now = current_timestamp();
    let expiry = now + (validity_hours * 3600);
    
    let capabilities = vec![
        Capability {
            with_resource: format!("iroh:namespace:{}", namespace_id),
            can: "iroh/docs/write".to_string(),
            nb: Some(serde_json::json!({
                "key_prefix": ["trust:", "claim:", "pointer:"],
                "max_entries": 10000,
            })),
        },
        Capability {
            with_resource: format!("iroh:namespace:{}", namespace_id),
            can: "iroh/docs/read".to_string(),
            nb: Some(serde_json::json!({
                "key_prefix": ["trust:", "claim:", "pointer:"],
            })),
        },
        Capability {
            with_resource: format!("iroh:namespace:{}", namespace_id),
            can: "iroh/docs/sync".to_string(),
            nb: Some(serde_json::json!({
                "peers": ["*"],
                "relay": true,
            })),
        },
    ];
    
    UcanBuilder::default()
        .issued_by(&identity_did)
        .for_audience(&transport_did)
        .with_subject(&identity_did)
        .not_before(now)
        .expires_at(expiry)
        .with_capabilities(capabilities)
        .with_nonce(generate_nonce())
        .with_meta("delegation_purpose", "identity_transport_separation")
        .with_meta("wallet_version", "1.0.0")
        .sign(identity_key)
}

/// Import and verify a delegation bundle
pub fn import_delegation(
    transport_key: &SigningKey,
    bundle: DelegationBundle,
) -> Result<ImportedDelegation> {
    // 1. Verify UCAN signature
    let identity_pubkey = resolve_did_to_key(&bundle.identity_did)?;
    bundle.ucan.verify(&identity_pubkey)?;
    
    // 2. Verify audience matches transport key
    let transport_did = did_from_signing_key(transport_key);
    if bundle.ucan.audience() != &transport_did {
        return Err(anyhow!("UCAN audience does not match transport key"));
    }
    
    // 3. Verify time bounds
    let now = current_timestamp();
    if let Some(nbf) = bundle.ucan.nbf {
        if now < nbf {
            return Err(anyhow!("UCAN not yet valid"));
        }
    }
    if let Some(exp) = bundle.ucan.exp {
        if now > exp {
            return Err(anyhow!("UCAN expired"));
        }
    }
    
    // 4. Verify author matches transport key
    let expected_author = Author::from(transport_key.clone());
    if bundle.author.id() != expected_author.id() {
        return Err(anyhow!("Author does not match transport key"));
    }
    
    Ok(ImportedDelegation {
        ucan: bundle.ucan,
        namespace: bundle.namespace_secret,
        author: bundle.author,
        identity_did: bundle.identity_did,
        imported_at: now,
    })
}

// ============================================================================
// SECTION 5: Content Types
// ============================================================================

/// CRDT metadata types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrdtMetadata {
    LwwRegister { version: u64 },
    OrSet { add_wins: bool, dot: Dot },
    MvRegister { timestamps: Vec<u64> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dot {
    pub replica_id: String,
    pub sequence: u64,
}

/// Identity content structure stored in iroh-docs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityContent {
    /// The actual value
    pub value: serde_json::Value,
    /// CRDT metadata
    pub crdt_meta: CrdtMetadata,
    /// CID of UCAN proving authority
    pub authority_cid: Cid,
    /// Proof of possession signature
    pub proof_signature: SignatureBytes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureBytes(pub Vec<u8>);

impl From<Signature> for SignatureBytes {
    fn from(sig: Signature) -> Self {
        Self(sig.to_bytes().to_vec())
    }
}

// ============================================================================
// SECTION 6: Validation
// ============================================================================

/// Errors during delegation operations
#[derive(Debug, thiserror::Error)]
pub enum DelegationError {
    #[error("Invalid UCAN signature")]
    InvalidSignature,
    #[error("UCAN expired")]
    Expired,
    #[error("UCAN not yet valid")]
    NotYetValid,
    #[error("Wrong audience: expected {expected}, got {actual}")]
    WrongAudience { expected: String, actual: String },
    #[error("Capability not authorized: {0}")]
    CapabilityNotAuthorized(String),
    #[error("Key prefix not allowed: {0}")]
    KeyPrefixNotAllowed(String),
}

/// Sync authority proof bundle
#[derive(Debug, Clone)]
pub struct SyncAuthorityProof {
    pub ucan: Ucan,
    pub namespace_secret: NamespaceSecret,
    pub author: Author,
    pub proof_of_possession: Signature,
    pub timestamp: u64,
}

impl SyncAuthorityProof {
    pub fn verify(&self, identity_did: &Did) -> Result<()> {
        // 1. Verify UCAN
        let identity_pubkey = resolve_did_to_key(identity_did)?;
        self.ucan.verify(&identity_pubkey)?;
        
        // 2. Verify issuer matches expected identity
        if self.ucan.issuer() != identity_did {
            return Err(anyhow!("UCAN issuer does not match expected identity"));
        }
        
        // 3. Verify audience matches author
        let author_did = did_from_verifying_key(&self.author.public_key());
        if self.ucan.audience() != &author_did {
            return Err(anyhow!("UCAN audience does not match author"));
        }
        
        // 4. Verify capabilities
        self.verify_capabilities()?;
        
        // 5. Verify proof of possession
        let message = format!("iroh-docs-auth:{}", self.timestamp);
        self.author.verify(message.as_bytes(), &self.proof_of_possession)?;
        
        // 6. Verify timestamp freshness (±60 seconds)
        let now = current_timestamp();
        if self.timestamp.abs_diff(now) > 60 {
            return Err(anyhow!("Proof timestamp too old"));
        }
        
        Ok(())
    }
    
    fn verify_capabilities(&self) -> Result<()> {
        // Verify that the UCAN has the required capabilities
        let required_caps = ["iroh/docs/write", "iroh/docs/read", "iroh/docs/sync"];
        
        for required in &required_caps {
            if !self.ucan.att.iter().any(|cap| &cap.can == *required) {
                return Err(anyhow!("Missing capability: {}", required));
            }
        }
        
        Ok(())
    }
}

/// Validator for sync operations
pub struct SyncValidator {
    identity_did: Did,
    revocation_list: HashSet<Cid>,
    clock_skew_tolerance: u64,
}

impl SyncValidator {
    pub fn new(identity_did: Did) -> Self {
        Self {
            identity_did,
            revocation_list: HashSet::new(),
            clock_skew_tolerance: 60,
        }
    }
    
    pub fn add_revocation(&mut self, cid: Cid) {
        self.revocation_list.insert(cid);
    }
    
    pub fn validate_sync_proof(&self, proof: &SyncAuthorityProof) -> Result<()> {
        // Check if UCAN is revoked
        let ucan_cid = proof.ucan.to_cid();
        if self.revocation_list.contains(&ucan_cid) {
            return Err(anyhow!("UCAN has been revoked"));
        }
        
        // Verify the proof
        proof.verify(&self.identity_did)
    }
}

// ============================================================================
// SECTION 7: Revocation
// ============================================================================

/// Revocation UCAN structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationUcan {
    pub ucv: String,
    pub iss: Did,
    pub aud: Did,
    pub sub: Did,
    pub nbf: u64,
    pub exp: Option<u64>,
    pub att: Vec<RevocationCapability>,
    pub prf: Vec<Cid>,
    pub sig: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationCapability {
    pub with_resource: String,
    pub can: String,
    pub nb: RevocationConstraints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationConstraints {
    pub target_ucan_cid: Cid,
    pub reason: String,
}

/// Create a revocation UCAN
pub fn create_revocation_ucan(
    identity_key: &SigningKey,
    transport_did: &Did,
    target_ucan_cid: Cid,
    reason: impl Into<String>,
) -> Result<RevocationUcan> {
    let identity_did = did_from_signing_key(identity_key);
    let now = current_timestamp();
    
    let mut revocation = RevocationUcan {
        ucv: "1.0.0-rc.1".to_string(),
        iss: identity_did.clone(),
        aud: transport_did.clone(),
        sub: identity_did,
        nbf: now,
        exp: None, // Permanent revocation
        att: vec![RevocationCapability {
            with_resource: "ucan:revoke".to_string(),
            can: "ucan/revoke".to_string(),
            nb: RevocationConstraints {
                target_ucan_cid,
                reason: reason.into(),
            },
        }],
        prf: vec![],
        sig: None,
    };
    
    // Sign
    let message = serde_json::to_vec(&revocation)?;
    let signature = identity_key.sign(&message);
    revocation.sig = Some(base64_encode(signature.to_bytes().as_slice()));
    
    Ok(revocation)
}

// ============================================================================
// SECTION 8: Helper Functions
// ============================================================================

/// Convert signing key to did:key format
pub fn did_from_signing_key(key: &SigningKey) -> Did {
    did_from_verifying_key(&key.verifying_key())
}

/// Convert verifying key to did:key format
pub fn did_from_verifying_key(key: &VerifyingKey) -> Did {
    // did:key:z6Mk prefix for Ed25519
    Did::new(format!("did:key:z6Mk{}", base58_encode(&key.to_bytes())))
}

/// Resolve DID to public key (simplified - in production use DID resolver)
pub fn resolve_did_to_key(did: &Did) -> Result<VerifyingKey> {
    let did_str = did.as_str();
    if !did_str.starts_with("did:key:z6Mk") {
        return Err(anyhow!("Unsupported DID method: {}", did_str));
    }
    
    let key_part = &did_str[12..]; // After "did:key:z6Mk"
    let key_bytes = base58_decode(key_part)?;
    
    if key_bytes.len() != 32 {
        return Err(anyhow!("Invalid key length: {}", key_bytes.len()));
    }
    
    let key_array: [u8; 32] = key_bytes.try_into()
        .map_err(|_| anyhow!("Failed to convert key bytes"))?;
    
    VerifyingKey::from_bytes(&key_array)
        .map_err(|e| anyhow!("Invalid verifying key: {:?}", e))
}

/// Generate cryptographically secure nonce
pub fn generate_nonce() -> Vec<u8> {
    use rand::RngCore;
    let mut nonce = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce);
    nonce.to_vec()
}

/// Get current Unix timestamp
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Base58 encode bytes
pub fn base58_encode(bytes: &[u8]) -> String {
    bs58::encode(bytes).into_string()
}

/// Base58 decode string
pub fn base58_decode(s: &str) -> Result<Vec<u8>> {
    bs58::decode(s).into_vec()
        .map_err(|e| anyhow!("Base58 decode error: {}", e))
}

/// Base64 encode bytes
pub fn base64_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Base64 decode string
pub fn base64_decode(s: &str) -> Result<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(s)
        .map_err(|e| anyhow!("Base64 decode error: {}", e))
}

/// Generate a new transport key
pub fn generate_transport_key() -> SigningKey {
    SigningKey::generate(&mut rand::thread_rng())
}

// ============================================================================
// SECTION 9: Example Usage
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_delegation_flow() {
        // 1. Generate Identity Key (represents user's master identity)
        let identity_key = SigningKey::generate(&mut rand::thread_rng());
        
        // 2. Generate Transport Key (ephemeral, per-device)
        let transport_key = SigningKey::generate(&mut rand::thread_rng());
        
        // 3. Create iroh-docs namespace
        let namespace = NamespaceSecret::new(&mut rand::thread_rng());
        
        // 4. Create UCAN delegation
        let ucan = create_delegation_ucan(
            &identity_key,
            &transport_key,
            &namespace,
            48, // 48 hours
        ).unwrap();
        
        // 5. Create author from transport key
        let author = Author::from(transport_key.clone());
        
        // 6. Create delegation bundle
        let bundle = DelegationBundle {
            ucan: ucan.clone(),
            namespace_secret: namespace.clone(),
            author: author.clone(),
            identity_did: did_from_signing_key(&identity_key),
        };
        
        // 7. Import and verify delegation
        let imported = import_delegation(&transport_key, bundle).unwrap();
        
        // 8. Verify imported delegation
        assert_eq!(imported.ucan.issuer().as_str(), did_from_signing_key(&identity_key).as_str());
        assert_eq!(imported.author.id().0, author.id().0);
        
        println!("✓ Delegation flow test passed");
    }
    
    #[test]
    fn test_ucan_verification() {
        let identity_key = SigningKey::generate(&mut rand::thread_rng());
        let transport_key = SigningKey::generate(&mut rand::thread_rng());
        let namespace = NamespaceSecret::new(&mut rand::thread_rng());
        
        let ucan = create_delegation_ucan(
            &identity_key,
            &transport_key,
            &namespace,
            48,
        ).unwrap();
        
        // Verify with correct key
        let result = ucan.verify(&identity_key.verifying_key());
        assert!(result.is_ok());
        
        // Verify with wrong key should fail
        let wrong_key = SigningKey::generate(&mut rand::thread_rng());
        let result = ucan.verify(&wrong_key.verifying_key());
        assert!(result.is_err());
        
        println!("✓ UCAN verification test passed");
    }
    
    #[test]
    fn test_revocation() {
        let identity_key = SigningKey::generate(&mut rand::thread_rng());
        let transport_key = SigningKey::generate(&mut rand::thread_rng());
        let namespace = NamespaceSecret::new(&mut rand::thread_rng());
        
        let ucan = create_delegation_ucan(
            &identity_key,
            &transport_key,
            &namespace,
            48,
        ).unwrap();
        
        let ucan_cid = ucan.to_cid();
        let transport_did = did_from_signing_key(&transport_key);
        
        let revocation = create_revocation_ucan(
            &identity_key,
            &transport_did,
            ucan_cid,
            "Device lost",
        ).unwrap();
        
        assert_eq!(revocation.att[0].nb.target_ucan_cid, ucan_cid);
        
        println!("✓ Revocation test passed");
    }
}

// ============================================================================
// SECTION 10: Integration with iroh-docs
// ============================================================================

/// Trait for iroh-docs store operations
#[async_trait]
pub trait IdentityStore: Send + Sync {
    /// Insert a new entry with proper capability proof
    async fn insert_identity_entry(
        &mut self,
        delegation: &ImportedDelegation,
        key: &str,
        value: serde_json::Value,
    ) -> Result<Entry>;
    
    /// Query entries by key prefix
    async fn query_by_prefix(
        &self,
        prefix: &str,
    ) -> Result<Vec<Entry>>;
    
    /// Verify entry authority
    async fn verify_entry(&self, entry: &Entry) -> Result<bool>;
}

/// Entry structure (mirrors iroh-docs Entry)
#[derive(Debug, Clone)]
pub struct Entry {
    pub key: String,
    pub author: AuthorId,
    pub namespace: NamespaceId,
    pub content_hash: Hash,
    pub content_size: u64,
    pub timestamp: u64,
    pub signature: Signature,
}

/// Example implementation of identity store operations
pub struct IdentityStoreImpl {
    delegation: ImportedDelegation,
    entries: Vec<Entry>,
}

impl IdentityStoreImpl {
    pub fn new(delegation: ImportedDelegation) -> Self {
        Self {
            delegation,
            entries: Vec::new(),
        }
    }
    
    /// Create proof of possession for a timestamp
    fn create_proof_of_possession(&self, timestamp: u64) -> Signature {
        let message = format!("iroh-docs-auth:{}", timestamp);
        self.delegation.author.sign(message.as_bytes())
    }
}

#[async_trait]
impl IdentityStore for IdentityStoreImpl {
    async fn insert_identity_entry(
        &mut self,
        delegation: &ImportedDelegation,
        key: &str,
        value: serde_json::Value,
    ) -> Result<Entry> {
        // Verify key prefix is allowed
        let allowed_prefixes = ["trust:", "claim:", "pointer:"];
        if !allowed_prefixes.iter().any(|p| key.starts_with(p)) {
            return Err(anyhow!("Key prefix not allowed: {}", key));
        }
        
        let timestamp = current_timestamp();
        
        // Create content
        let content = IdentityContent {
            value,
            crdt_meta: CrdtMetadata::LwwRegister { version: 1 },
            authority_cid: delegation.ucan.to_cid(),
            proof_signature: self.create_proof_of_possession(timestamp).into(),
        };
        
        // Serialize and hash
        let content_bytes = serde_json::to_vec(&content)?;
        let content_hash = blake3::hash(&content_bytes);
        
        // Sign entry
        let entry_data = format!("{}:{}:{}:{}", key, timestamp, content_hash, content_bytes.len());
        let signature = delegation.author.sign(entry_data.as_bytes());
        
        let entry = Entry {
            key: key.to_string(),
            author: delegation.author.id(),
            namespace: delegation.namespace.id(),
            content_hash,
            content_size: content_bytes.len() as u64,
            timestamp,
            signature,
        };
        
        self.entries.push(entry.clone());
        
        Ok(entry)
    }
    
    async fn query_by_prefix(&self, prefix: &str) -> Result<Vec<Entry>> {
        Ok(self.entries
            .iter()
            .filter(|e| e.key.starts_with(prefix))
            .cloned()
            .collect())
    }
    
    async fn verify_entry(&self, entry: &Entry) -> Result<bool> {
        // Verify author matches
        if entry.author != self.delegation.author.id() {
            return Ok(false);
        }
        
        // Verify namespace matches
        if entry.namespace != self.delegation.namespace.id() {
            return Ok(false);
        }
        
        // Verify signature
        let entry_data = format!("{}:{}:{}:{}", entry.key, entry.timestamp, entry.content_hash, entry.content_size);
        match self.delegation.author.verify(entry_data.as_bytes(), &entry.signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

// ============================================================================
// Dependencies (add to Cargo.toml):
// ============================================================================
// [dependencies]
// anyhow = "1.0"
// async-trait = "0.1"
// ed25519-dalek = "2.0"
// serde = { version = "1.0", features = ["derive"] }
// serde_json = "1.0"
// blake3 = "1.5"
// cid = "0.11"
// bs58 = "0.5"
// base64 = "0.21"
// rand = "0.8"
// thiserror = "1.0"
// tokio = { version = "1.0", features = ["full"] }
// iroh-docs = "0.96"
// ============================================================================
