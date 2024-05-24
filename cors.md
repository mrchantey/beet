
view:
gcloud storage buckets describe gs://beet-examples --format="default(cors_config)"
update:
gcloud storage buckets update gs://beet-examples --cors-file=cors.json