-- sqlx does not auto wrap in a transaction so we do manually
BEGIN;
  UPDATE subscriptions
    SET status = 'confirmed'
    WHERE status IS NULL;
  ALTER TABLE subscriptions ALTER COLUMN status SET NOT NULL;
COMMIT;
