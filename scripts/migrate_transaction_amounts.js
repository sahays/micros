// MongoDB Migration: Convert transaction amounts from rupees (Double) to paise (Int64)
// and rename the field from `amount` to `amount_paise`.
//
// Story 011: Standardize Amount Units to Paise
//
// Usage:
//   mongosh "mongodb://localhost:27017/payment_db" scripts/migrate_transaction_amounts.js
//
// Phase 1: Converts legacy Double `amount` (rupees) → Int64 `amount_paise` (paise).
// Phase 2: Renames already-integer `amount` → `amount_paise` for documents that
//          were created after the uint64 change but before the rename.
//
// The script is idempotent: running it multiple times is safe.

const DB_NAME = "payment_db";
const COLLECTION = "transactions";
const BATCH_SIZE = 500;

const db = db.getSiblingDB(DB_NAME);
const collection = db.getCollection(COLLECTION);

let updated = 0;
let skipped = 0;
let errors = 0;

// =========================================================================
// Phase 1: Convert Double `amount` (rupees) → Int64 `amount_paise` (paise)
// =========================================================================
print(`Phase 1: Converting Double amounts from rupees to paise...`);

const doubleCursor = collection.find({ amount: { $type: "double" } }).batchSize(BATCH_SIZE);

while (doubleCursor.hasNext()) {
  const doc = doubleCursor.next();
  const oldAmount = doc.amount;
  const newAmount = NumberLong(Math.round(oldAmount * 100));

  try {
    const result = collection.updateOne(
      { _id: doc._id },
      {
        $set: { amount_paise: newAmount, updated_at: new Date() },
        $unset: { amount: "" }
      }
    );

    if (result.modifiedCount === 1) {
      updated++;
    } else {
      skipped++;
    }
  } catch (e) {
    errors++;
    print(`ERROR updating document ${doc._id}: ${e.message}`);
  }

  if (updated % 1000 === 0 && updated > 0) {
    print(`  Progress: ${updated} documents updated...`);
  }
}

print(`  Phase 1 complete: ${updated} converted, ${skipped} skipped, ${errors} errors`);

// =========================================================================
// Phase 2: Rename integer `amount` → `amount_paise` (no value conversion)
// =========================================================================
print(`Phase 2: Renaming remaining 'amount' fields to 'amount_paise'...`);

const renameResult = collection.updateMany(
  { amount: { $exists: true }, amount_paise: { $exists: false } },
  { $rename: { amount: "amount_paise" } }
);

print(`  Phase 2 complete: ${renameResult.modifiedCount} documents renamed`);

// =========================================================================
// Verification
// =========================================================================
const remainingOldField = collection.countDocuments({ amount: { $exists: true } });
const totalNewField = collection.countDocuments({ amount_paise: { $exists: true } });

print(`\nMigration complete.`);
print(`  Documents with 'amount' (old): ${remainingOldField}`);
print(`  Documents with 'amount_paise' (new): ${totalNewField}`);

if (remainingOldField > 0) {
  print("WARNING: Some documents still have the old 'amount' field.");
} else {
  print("SUCCESS: All amounts are now stored as 'amount_paise' (Int64 paise).");
}
