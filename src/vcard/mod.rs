pub mod model;
pub mod parser;
pub mod serializer;

pub use model::{
    validate_email, validate_full_name, validate_phone, validate_postal_code, validate_url,
    Address, TypedValue, VCard, MAX_FULL_NAME_LEN,
};
pub use parser::parse_vcf;
pub use serializer::serialize_vcf;
#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_VCF: &str = "\
BEGIN:VCARD
VERSION:3.0
N:Santoso;Budi;;;
FN:Budi Santoso
ORG:PT Maju Jaya Teknologi
TITLE:Manajer Pemasaran
TEL;TYPE=CELL,VOICE:+6281234567890
TEL;TYPE=WORK,VOICE:+62315551234
EMAIL;TYPE=PREF,INTERNET:budi.santoso@majujaya.co.id
URL:https://www.majujaya.co.id
ADR;TYPE=WORK:;;Jl. Pemuda No. 123;Surabaya;Jawa Timur;60271;Indonesia
NOTE:Kontak utama untuk kerja sama proyek digital.
END:VCARD
";

    #[test]
    fn test_parse_and_roundtrip_vcf_3_0() {
        let cards = parse_vcf(SAMPLE_VCF).expect("Harus berhasil parse VCF 3.0");
        assert_eq!(cards.len(), 1);

        let card = &cards[0];
        assert_eq!(card.n.family, "Santoso");
        assert_eq!(card.n.given, "Budi");
        assert_eq!(card.fn_name, "Budi Santoso");
        assert_eq!(card.org, "PT Maju Jaya Teknologi");
        assert_eq!(card.title, "Manajer Pemasaran");
        assert_eq!(card.phones.len(), 2);
        assert_eq!(card.phones[0].types, vec!["CELL", "VOICE"]);
        assert_eq!(card.phones[0].value, "+6281234567890");
        assert_eq!(card.emails[0].value, "budi.santoso@majujaya.co.id");
        assert_eq!(card.addresses[0].city, "Surabaya");
        assert_eq!(card.addresses[0].postal_code, "60271");

        let serialized = serialize_vcf(&cards);
        let reparsed = parse_vcf(&serialized).expect("Hasil serialize harus valid");
        assert_eq!(cards, reparsed);
    }

    #[test]
    fn test_reject_non_3_0_version() {
        let vcf_2_1 = "BEGIN:VCARD\nVERSION:2.1\nFN:Test\nEND:VCARD\n";
        let err = parse_vcf(vcf_2_1).unwrap_err();
        assert!(matches!(err, VCardError::UnsupportedVersion { .. }));
    }
}

