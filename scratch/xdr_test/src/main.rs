use stellar_xdr::curr::TransactionEnvelope;

fn main() {

    let env = TransactionEnvelope::Tx(todo!());
    match env {
        TransactionEnvelope::Tx(ref envelope) => {
            let _ = envelope.tx.fee;
        }
        TransactionEnvelope::TxFeeBump(ref envelope) => {
            let _ = envelope.tx.fee;
        }
        TransactionEnvelope::TxV0(ref envelope) => {
            let _ = envelope.tx.fee;
        }
    }
}
