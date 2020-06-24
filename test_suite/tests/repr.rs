#[allow(dead_code)]
mod tests {
    use opg::*;
    use serde::Serialize;

    #[derive(Serialize, Deserialize, OpgModel)]
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

    // TODO: add repr tests
}
