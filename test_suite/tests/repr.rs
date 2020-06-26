#[allow(dead_code)]
mod tests {
    use opg::*;
    use serde_repr::Serialize_repr;

    #[derive(Serialize_repr, OpgModel)]
    #[repr(i32)]
    pub enum LedgerAccountId {
        IssuedLoansAndCredits = 5586,
        LiabilitiesForSale = 4909,
        InterestRateOnIssuedLoansAndCredits = 55861,
        OtherIncomesFromCoreActivity = 819,
        FinesOnIssuedLoansAndCredits = 55862,
        InterestIncomes = 700,
        CurrentAccounts = 651,
        Pledge = 008,
    }

    #[test]
    fn test_repr() {
        let mut cx = &mut OpgComponents::default();
        assert_eq!(
            serde_yaml::to_string(&LedgerAccountId::get_structure(&mut cx)).unwrap(),
            r##"---
oneOf:
  - description: IssuedLoansAndCredits variant
    type: integer
    example: "5586"
  - description: LiabilitiesForSale variant
    type: integer
    example: "4909"
  - description: InterestRateOnIssuedLoansAndCredits variant
    type: integer
    example: "55861"
  - description: OtherIncomesFromCoreActivity variant
    type: integer
    example: "819"
  - description: FinesOnIssuedLoansAndCredits variant
    type: integer
    example: "55862"
  - description: InterestIncomes variant
    type: integer
    example: "700"
  - description: CurrentAccounts variant
    type: integer
    example: "651"
  - description: Pledge variant
    type: integer
    example: "008""##
        );
    }
}
