# Cloud Build & Deployment Commands

## 1. Prerequisites

### 1.1 Required GCP APIs

```bash
gcloud services enable \
  cloudbuild.googleapis.com \
  run.googleapis.com \
  artifactregistry.googleapis.com \
  secretmanager.googleapis.com \
  vpcaccess.googleapis.com \
  --project=${PROJECT_ID}
```

### 1.2 Create Artifact Registry Repository

```bash
gcloud artifacts repositories create agentics \
  --repository-format=docker \
  --location=${REGION} \
  --description="Agentics platform container images" \
  --project=${PROJECT_ID}
```

### 1.3 Create VPC Connector

```bash
gcloud compute networks vpc-access connectors create agentics-vpc \
  --region=${REGION} \
  --network=default \
  --range=10.8.0.0/28 \
  --project=${PROJECT_ID}
```

### 1.4 Create Service Account

```bash
# Create service account
gcloud iam service-accounts create llm-incident-manager \
  --display-name="LLM Incident Manager Service Account" \
  --project=${PROJECT_ID}

# Grant minimal required roles
gcloud projects add-iam-policy-binding ${PROJECT_ID} \
  --member="serviceAccount:llm-incident-manager@${PROJECT_ID}.iam.gserviceaccount.com" \
  --role="roles/run.invoker"

gcloud projects add-iam-policy-binding ${PROJECT_ID} \
  --member="serviceAccount:llm-incident-manager@${PROJECT_ID}.iam.gserviceaccount.com" \
  --role="roles/secretmanager.secretAccessor"

gcloud projects add-iam-policy-binding ${PROJECT_ID} \
  --member="serviceAccount:llm-incident-manager@${PROJECT_ID}.iam.gserviceaccount.com" \
  --role="roles/logging.logWriter"

gcloud projects add-iam-policy-binding ${PROJECT_ID} \
  --member="serviceAccount:llm-incident-manager@${PROJECT_ID}.iam.gserviceaccount.com" \
  --role="roles/monitoring.metricWriter"
```

### 1.5 Create Secrets

```bash
# Create RuVector API key secret
echo -n "your-ruvector-api-key" | gcloud secrets create ruvector-api-key-dev \
  --data-file=- \
  --replication-policy=automatic \
  --project=${PROJECT_ID}

# For staging
echo -n "your-ruvector-api-key-staging" | gcloud secrets create ruvector-api-key-staging \
  --data-file=- \
  --replication-policy=automatic \
  --project=${PROJECT_ID}

# For production
echo -n "your-ruvector-api-key-prod" | gcloud secrets create ruvector-api-key-prod \
  --data-file=- \
  --replication-policy=automatic \
  --project=${PROJECT_ID}

# Grant access to service account
gcloud secrets add-iam-policy-binding ruvector-api-key-dev \
  --member="serviceAccount:llm-incident-manager@${PROJECT_ID}.iam.gserviceaccount.com" \
  --role="roles/secretmanager.secretAccessor" \
  --project=${PROJECT_ID}
```

## 2. Deployment Commands

### 2.1 Development Environment

```bash
# Using Cloud Build
gcloud builds submit \
  --config=deploy/cloudbuild.yaml \
  --substitutions=_ENV=dev,_REGION=us-central1 \
  --project=${PROJECT_ID}

# Or using gcloud run deploy directly
gcloud run deploy llm-incident-manager \
  --source=. \
  --region=us-central1 \
  --platform=managed \
  --min-instances=1 \
  --max-instances=5 \
  --memory=2Gi \
  --cpu=2 \
  --port=8080 \
  --ingress=internal-and-cloud-load-balancing \
  --vpc-connector=agentics-vpc \
  --service-account=llm-incident-manager@${PROJECT_ID}.iam.gserviceaccount.com \
  --set-env-vars="SERVICE_NAME=llm-incident-manager,PLATFORM_ENV=dev" \
  --set-secrets="RUVECTOR_API_KEY=ruvector-api-key-dev:latest" \
  --no-allow-unauthenticated \
  --project=${PROJECT_ID}
```

### 2.2 Staging Environment

```bash
gcloud builds submit \
  --config=deploy/cloudbuild.yaml \
  --substitutions=_ENV=staging,_REGION=us-central1,_MIN_INSTANCES=2,_MAX_INSTANCES=8 \
  --project=${PROJECT_ID}
```

### 2.3 Production Environment

```bash
gcloud builds submit \
  --config=deploy/cloudbuild.yaml \
  --substitutions=_ENV=prod,_REGION=us-central1,_MIN_INSTANCES=3,_MAX_INSTANCES=10,_MEMORY=4Gi \
  --project=${PROJECT_ID}
```

### 2.4 Using Service YAML

```bash
# Substitute environment variables and apply
export PROJECT_ID=your-project
export REGION=us-central1
export ENV=prod
export TAG=v1.0.1

envsubst < deploy/service.yaml | gcloud run services replace - --region=${REGION}
```

## 3. IAM Requirements (Least Privilege)

### 3.1 Service Account Roles

| Role | Purpose |
|------|---------|
| `roles/run.invoker` | Invoke other Cloud Run services (ruvector, observatory) |
| `roles/secretmanager.secretAccessor` | Read secrets (API keys) |
| `roles/logging.logWriter` | Write logs to Cloud Logging |
| `roles/monitoring.metricWriter` | Write metrics to Cloud Monitoring |

### 3.2 Cloud Build Service Account Roles

| Role | Purpose |
|------|---------|
| `roles/run.admin` | Deploy Cloud Run services |
| `roles/artifactregistry.writer` | Push container images |
| `roles/iam.serviceAccountUser` | Act as service account |
| `roles/secretmanager.viewer` | List secrets (for validation) |

```bash
# Grant Cloud Build service account permissions
PROJECT_NUMBER=$(gcloud projects describe ${PROJECT_ID} --format='value(projectNumber)')

gcloud projects add-iam-policy-binding ${PROJECT_ID} \
  --member="serviceAccount:${PROJECT_NUMBER}@cloudbuild.gserviceaccount.com" \
  --role="roles/run.admin"

gcloud projects add-iam-policy-binding ${PROJECT_ID} \
  --member="serviceAccount:${PROJECT_NUMBER}@cloudbuild.gserviceaccount.com" \
  --role="roles/artifactregistry.writer"

gcloud iam service-accounts add-iam-policy-binding \
  llm-incident-manager@${PROJECT_ID}.iam.gserviceaccount.com \
  --member="serviceAccount:${PROJECT_NUMBER}@cloudbuild.gserviceaccount.com" \
  --role="roles/iam.serviceAccountUser" \
  --project=${PROJECT_ID}
```

## 4. Networking Requirements

### 4.1 Internal Invocation Only

LLM-Incident-Manager is configured with `--ingress=internal-and-cloud-load-balancing`:

- Can ONLY be invoked from:
  - Other Cloud Run services in the same project
  - VPC networks via VPC connector
  - Cloud Load Balancing

- CANNOT be invoked from:
  - Public internet
  - External services

### 4.2 VPC Connector Configuration

```bash
# List VPC connectors
gcloud compute networks vpc-access connectors list --region=${REGION}

# Describe connector
gcloud compute networks vpc-access connectors describe agentics-vpc \
  --region=${REGION}
```

### 4.3 Service-to-Service Authentication

When LLM-Orchestrator calls LLM-Incident-Manager:

```bash
# Get identity token for calling internal services
TOKEN=$(gcloud auth print-identity-token \
  --audiences="https://llm-incident-manager-${PROJECT_ID}.run.app")

# Call the service
curl -H "Authorization: Bearer ${TOKEN}" \
  https://llm-incident-manager-${PROJECT_ID}.run.app/health
```

## 5. Verification Commands

### 5.1 Check Deployment Status

```bash
# List revisions
gcloud run revisions list \
  --service=llm-incident-manager \
  --region=${REGION}

# Describe service
gcloud run services describe llm-incident-manager \
  --region=${REGION}

# Get service URL
gcloud run services describe llm-incident-manager \
  --region=${REGION} \
  --format='value(status.url)'
```

### 5.2 Check Logs

```bash
# Stream logs
gcloud logging read "resource.type=cloud_run_revision AND resource.labels.service_name=llm-incident-manager" \
  --project=${PROJECT_ID} \
  --limit=50 \
  --format="table(timestamp,severity,textPayload)"

# Stream live logs
gcloud alpha run services logs tail llm-incident-manager \
  --region=${REGION}
```

### 5.3 Check Health

```bash
# Get identity token
TOKEN=$(gcloud auth print-identity-token)

# Health check
curl -H "Authorization: Bearer ${TOKEN}" \
  "$(gcloud run services describe llm-incident-manager --region=${REGION} --format='value(status.url)')/health"

# Readiness check
curl -H "Authorization: Bearer ${TOKEN}" \
  "$(gcloud run services describe llm-incident-manager --region=${REGION} --format='value(status.url)')/health/ready"
```

## 6. Rollback Commands

### 6.1 Rollback to Previous Revision

```bash
# List revisions
gcloud run revisions list \
  --service=llm-incident-manager \
  --region=${REGION}

# Route traffic to previous revision
gcloud run services update-traffic llm-incident-manager \
  --region=${REGION} \
  --to-revisions=llm-incident-manager-00002-abc=100
```

### 6.2 Emergency Rollback

```bash
# Set traffic to known good revision
gcloud run services update-traffic llm-incident-manager \
  --region=${REGION} \
  --to-tags=stable=100
```
