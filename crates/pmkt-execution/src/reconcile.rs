/// Reconciliation status.
#[derive(Debug, Clone, Default)]
pub struct ReconciliationStatus {
    pub complete: bool,
    pub open_orders_confirmed: bool,
    pub positions_confirmed: bool,
    pub notes: Vec<String>,
}

impl ReconciliationStatus {
    /// Stub: In v1, this performs local-state-only reconciliation.
    /// In a real implementation, this would query the exchange.
    pub fn perform_stub() -> Self {
        ReconciliationStatus {
            complete: true,
            open_orders_confirmed: true,
            positions_confirmed: true,
            notes: vec!["stub reconciliation: no exchange query in v1".to_string()],
        }
    }
}
