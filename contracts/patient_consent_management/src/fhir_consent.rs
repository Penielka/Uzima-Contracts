//! FHIR R4 Consent Resource Mapping
//!
//! Maps the internal ConsentRecord structure to the HL7 FHIR R4 Consent resource schema.
//! Provides a `to_fhir_consent` conversion utility and structured output for interoperability
//! with EHR systems that consume FHIR R4 resources.
//!
//! Reference: https://www.hl7.org/fhir/R4/consent.html

use soroban_sdk::{contracttype, Address, Env, String, Vec};

use crate::ConsentRecord;

/// FHIR R4 Consent status values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum FhirConsentStatus {
    /// The consent is active and in effect.
    Active,
    /// The consent has been revoked or has expired.
    Inactive,
    /// The consent is proposed but not yet active.
    Proposed,
    /// The consent was rejected before becoming active.
    Rejected,
    /// The consent has been entered in error.
    EnteredInError,
}

/// FHIR R4 Provision type (permit or deny).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum FhirProvisionType {
    Permit,
    Deny,
}

/// FHIR R4 CodeableConcept — a coding with system, code, and display.
#[derive(Clone)]
#[contracttype]
pub struct FhirCoding {
    pub system: String,
    pub code: String,
    pub display: String,
}

/// FHIR R4 Reference — a reference to another resource (e.g. Patient/123, Practitioner/456).
#[derive(Clone)]
#[contracttype]
pub struct FhirReference {
    pub reference: String,
    pub display: String,
}

/// FHIR R4 Consent.provision.actor — who is granted access.
#[derive(Clone)]
#[contracttype]
pub struct FhirProvisionActor {
    pub role: Vec<FhirCoding>,
    pub reference: FhirReference,
}

/// FHIR R4 Consent.provision — what the consent allows or denies.
#[derive(Clone)]
#[contracttype]
pub struct FhirProvision {
    /// "permit" or "deny"
    pub provision_type: FhirProvisionType,
    /// Period start (Unix epoch seconds).
    pub period_start: u64,
    /// Period end (Unix epoch seconds); 0 = no end.
    pub period_end: u64,
    /// Who this provision applies to (the data recipient).
    pub actor: Vec<FhirProvisionActor>,
    /// Purpose of use codings.
    pub purpose: Vec<FhirCoding>,
}

/// FHIR R4 Consent resource.
///
/// Maps to: https://www.hl7.org/fhir/R4/consent.html
#[derive(Clone)]
#[contracttype]
pub struct FhirConsent {
    /// Always "Consent"
    pub resource_type: String,
    /// active | inactive | proposed | rejected | entered-in-error
    pub status: FhirConsentStatus,
    /// Scope of the consent: always "patient-privacy" for healthcare data access.
    pub scope: FhirCoding,
    /// Category of consent: "IDSCL" (information disclosure).
    pub category: Vec<FhirCoding>,
    /// The patient this consent is about.
    pub patient: FhirReference,
    /// When the consent was given (Unix epoch seconds).
    pub date_time: u64,
    /// The patient who granted the consent (same as patient for self-consent).
    pub performer: Vec<FhirReference>,
    /// The provision (access rules) this consent grants.
    pub provision: FhirProvision,
}

// ── FHIR Consent Mapping ─────────────────────────────────────────────────────

/// Maps the internal `ConsentRecord` to a FHIR R4 `FhirConsent` resource.
///
/// # Mapping Reference
///
/// | ConsentRecord Field | FHIR R4 Consent Field         | Notes                                |
/// |---------------------|-------------------------------|--------------------------------------|
/// | `patient`           | `patient.reference`           | The patient this consent is about    |
/// | `provider`          | `provision.actor[0].reference`| The healthcare provider granted access|
/// | `granted_at`        | `date_time`                   | When consent was given (Unix epoch)  |
/// | `expires_at`        | `provision.period_end`        | 0 means no expiration                |
/// | `active`            | `status`                      | Active → Active, Inactive → Inactive |
/// | `revoked_at`        | (implicit via status)         | Credentials revoked if not active    |
///
/// # Example
///
/// ```rust,ignore
/// let record = ConsentRecord { patient, provider, granted_at: ts, expires_at: 0, revoked_at: 0, active: true };
/// let fhir: FhirConsent = to_fhir_consent(&env, &record, "patient-address-string", "provider-address-string");
/// ```
pub fn to_fhir_consent(
    env: &Env,
    record: &ConsentRecord,
    patient_ref: &str,
    provider_ref: &str,
) -> FhirConsent {
    let status = if record.active {
        FhirConsentStatus::Active
    } else {
        FhirConsentStatus::Inactive
    };

    let scope = FhirCoding {
        system: String::from_str(
            env,
            "http://terminology.hl7.org/CodeSystem/consentscope",
        ),
        code: String::from_str(env, "patient-privacy"),
        display: String::from_str(env, "Privacy Consent"),
    };

    let category = Vec::from_array(
        env,
        &[FhirCoding {
            system: String::from_str(env, "http://loinc.org"),
            code: String::from_str(env, "59284-0"),
            display: String::from_str(env, "Patient Consent"),
        }],
    );

    let patient = FhirReference {
        reference: String::from_str(env, patient_ref),
        display: String::from_str(env, ""),
    };

    let performer = Vec::from_array(
        env,
        &[FhirReference {
            reference: String::from_str(env, patient_ref),
            display: String::from_str(env, ""),
        }],
    );

    let actor_role = Vec::from_array(
        env,
        &[FhirCoding {
            system: String::from_str(env, "http://terminology.hl7.org/CodeSystem/v3-ParticipationType"),
            code: String::from_str(env, "IRCP"),
            display: String::from_str(env, "information recipient"),
        }],
    );

    let actor = Vec::from_array(
        env,
        &[FhirProvisionActor {
            role: actor_role,
            reference: FhirReference {
                reference: String::from_str(env, provider_ref),
                display: String::from_str(env, ""),
            },
        }],
    );

    let purpose = Vec::from_array(
        env,
        &[FhirCoding {
            system: String::from_str(env, "http://terminology.hl7.org/CodeSystem/v3-ActReason"),
            code: String::from_str(env, "TREAT"),
            display: String::from_str(env, "treatment"),
        }],
    );

    let provision = FhirProvision {
        provision_type: FhirProvisionType::Permit,
        period_start: record.granted_at,
        period_end: record.expires_at,
        actor,
        purpose,
    };

    FhirConsent {
        resource_type: String::from_str(env, "Consent"),
        status,
        scope,
        category,
        patient,
        date_time: record.granted_at,
        performer,
        provision,
    }
}

/// Validates that a `FhirConsent` resource conforms to FHIR R4 schema requirements.
///
/// Checks that required fields are present (scope, patient reference, provision actors).
/// The `resource_type` is not compared across `Env` instances since `String` values
/// in Soroban SDK are scoped to the `Env` they were created in.
///
/// Returns `true` if the resource is valid, `false` otherwise.
pub fn validate_fhir_consent(resource: &FhirConsent) -> bool {
    // Scope must be present and non-empty
    if resource.scope.code.is_empty() {
        return false;
    }

    // Patient reference must be present
    if resource.patient.reference.is_empty() {
        return false;
    }

    // Provision must have at least one actor
    if resource.provision.actor.is_empty() {
        return false;
    }

    true
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod fhir_consent_tests {
    use super::*;
    use crate::ConsentRecord;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::Env;

    /// Build a test `ConsentRecord` representing an active, non-expiring consent.
    fn build_test_record(env: &Env) -> ConsentRecord {
        let patient = Address::generate(env);
        let provider = Address::generate(env);
        ConsentRecord {
            patient,
            provider,
            granted_at: 1700000000,
            expires_at: 0, // no expiration
            revoked_at: 0,
            active: true,
        }
    }

    /// Build a revoked `ConsentRecord`.
    fn build_revoked_record(env: &Env) -> ConsentRecord {
        let patient = Address::generate(env);
        let provider = Address::generate(env);
        ConsentRecord {
            patient,
            provider,
            granted_at: 1700000000,
            expires_at: 0,
            revoked_at: 1700100000,
            active: false,
        }
    }

    /// Build a consent with an expiry date set.
    fn build_expiry_record(env: &Env) -> ConsentRecord {
        let patient = Address::generate(env);
        let provider = Address::generate(env);
        ConsentRecord {
            patient,
            provider,
            granted_at: 1700000000,
            expires_at: 1800000000,
            revoked_at: 0,
            active: true,
        }
    }

    #[test]
    fn test_to_fhir_consent_active() {
        let env = Env::default();
        let record = build_test_record(&env);

        let fhir = to_fhir_consent(
            &env,
            &record,
            "Patient/P-00123",
            "Practitioner/PR-00456",
        );

        assert_eq!(fhir.resource_type, String::from_str(&env, "Consent"));
        assert_eq!(fhir.status, FhirConsentStatus::Active);
        assert_eq!(fhir.scope.code, String::from_str(&env, "patient-privacy"));
        assert_eq!(fhir.category.len(), 1);
        assert_eq!(
            fhir.category.first().unwrap().code,
            String::from_str(&env, "59284-0")
        );
        assert_eq!(
            fhir.patient.reference,
            String::from_str(&env, "Patient/P-00123")
        );
        assert_eq!(fhir.date_time, 1700000000);
        assert_eq!(
            fhir.performer.first().unwrap().reference,
            String::from_str(&env, "Patient/P-00123")
        );
        assert_eq!(fhir.provision.provision_type, FhirProvisionType::Permit);
        assert_eq!(fhir.provision.period_start, 1700000000);
        assert_eq!(fhir.provision.period_end, 0); // no expiry
        assert_eq!(fhir.provision.actor.len(), 1);
        assert_eq!(
            fhir.provision.actor.first().unwrap().reference.reference,
            String::from_str(&env, "Practitioner/PR-00456")
        );
        assert_eq!(
            fhir.provision.actor.first().unwrap().role.first().unwrap().code,
            String::from_str(&env, "IRCP")
        );
        assert_eq!(
            fhir.provision.purpose.first().unwrap().code,
            String::from_str(&env, "TREAT")
        );

        assert!(validate_fhir_consent(&fhir));
    }

    #[test]
    fn test_to_fhir_consent_revoked() {
        let env = Env::default();
        let record = build_revoked_record(&env);

        let fhir = to_fhir_consent(
            &env,
            &record,
            "Patient/P-00999",
            "Practitioner/PR-00888",
        );

        assert_eq!(fhir.status, FhirConsentStatus::Inactive);
        assert_eq!(
            fhir.patient.reference,
            String::from_str(&env, "Patient/P-00999")
        );
        assert_eq!(
            fhir.provision.actor.first().unwrap().reference.reference,
            String::from_str(&env, "Practitioner/PR-00888")
        );
        assert!(validate_fhir_consent(&fhir));
    }

    #[test]
    fn test_to_fhir_consent_with_expiry() {
        let env = Env::default();
        let record = build_expiry_record(&env);

        let fhir = to_fhir_consent(
            &env,
            &record,
            "Patient/P-00500",
            "Practitioner/PR-00700",
        );

        assert_eq!(fhir.status, FhirConsentStatus::Active);
        assert_eq!(fhir.provision.period_start, 1700000000);
        assert_eq!(fhir.provision.period_end, 1800000000); // expiry set
        assert!(validate_fhir_consent(&fhir));
    }

    #[test]
    fn test_validate_rejects_empty_patient_ref() {
        let env = Env::default();
        let record = build_test_record(&env);

        let mut fhir = to_fhir_consent(
            &env,
            &record,
            "Patient/P-00123",
            "Practitioner/PR-00456",
        );
        fhir.patient.reference = String::from_str(&env, "");

        assert!(!validate_fhir_consent(&fhir));
    }

    #[test]
    fn test_validate_rejects_empty_actors() {
        let env = Env::default();
        let record = build_test_record(&env);

        let mut fhir = to_fhir_consent(
            &env,
            &record,
            "Patient/P-00123",
            "Practitioner/PR-00456",
        );
        fhir.provision.actor = Vec::new(&env);

        assert!(!validate_fhir_consent(&fhir));
    }
}
