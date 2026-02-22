---
status: accepted
date: 2026-02-22
decision-makers: []
---

# ADR-0002: AWS Lambda and CDK as Inbound Adapter Entry Points

## Context and Problem Statement

ADR-0001 established a modular FCIS folder structure with a `shell/` as the composition root and a long-running process as the assumed runtime model. When deploying to AWS using Lambda and CDK, there is no long-running process — each Lambda invocation replaces the `main.rs` loop. We need to decide how Lambda entry points map to the existing structure without bleeding deployment concerns into use cases or duplicating wiring logic.

## Decision Drivers

- Lambda is a deployment detail — use cases must not know they run on Lambda
- Each Lambda needs its own bootstrap: config, infrastructure instantiation, wiring
- Bootstrap is shell responsibility, not use case responsibility
- The inbound adapter is the correct target for a Lambda handler — it translates the AWS event payload into a domain command or query
- CDK is infrastructure-as-code and belongs outside the application src tree
- Multiple deployment targets must be supported: Lambda for production, long-running local server for development

## Considered Options

1. Bootstrap inside each use case (`use_cases/create_time_entry/lambda.rs`)
2. One Lambda per use case, bootstrapped from a centralised `shell/lambdas/`
3. One Lambda for the entire bounded context, routing internally
4. Lambda adapter as a separate inbound adapter type per use case

## Decision Outcome

Chosen option 2: **One Lambda per use case, bootstrapped from a centralised `shell/lambdas/`**, because it maps naturally to the vertical slice structure, keeps deployment concerns out of use cases, and preserves the shell as the single place responsible for wiring — just spread across multiple entry points instead of one.

### Consequences

* Good, because use cases have zero knowledge of Lambda, CDK, or AWS — they remain deployment-agnostic
* Good, because one Lambda per use case maps naturally to the vertical slice structure — independent scaling and deployment per use case at no architectural cost
* Good, because shell remains the single place responsible for wiring — just distributed across multiple entry points
* Good, because CDK mirrors the use case structure making infrastructure changes predictable and navigable
* Good, because local development reuses the same inbound adapters with in-memory infrastructure — no mocking needed
* Good, because cold start performance is optimised per Lambda — each binary contains only what one use case needs
* Bad, because more Lambda functions to manage in CDK — offset by the predictable one-to-one mapping with use cases
* Bad, because config must be available as environment variables per Lambda — CDK is responsible for injecting the right values per function
* Bad, because bootstrap code is repeated across Lambda entry points — kept thin by convention, but requires discipline
* Bad, because shared infrastructure resources (DynamoDB tables, SQS queues) must be defined once and referenced by all Lambdas — CDK stack organisation must reflect this to avoid duplication
* Bad, because cold starts may be a concern for latency-sensitive use cases — consider provisioned concurrency for critical paths
* Bad, because if a use case needs both an API Gateway trigger and an SQS trigger, it needs two Lambda entry points in `shell/lambdas/` — this is correct but must be documented clearly to avoid confusion

### Confirmation

Compliance is confirmed by verifying that no use case source file imports `lambda_runtime`, `aws_lambda_events`, or any CDK type; all Lambda entry points live under `shell/lambdas/`; and `cdk/` is outside the `src/` tree.

## AWS Inbound Trigger Mapping

Different AWS services act as inbound triggers, each mapping to a specific inbound adapter type:

| AWS Trigger     | Inbound Adapter Type | Use Case Example          |
|-----------------|----------------------|---------------------------|
| API Gateway     | HTTP inbound adapter | `create_time_entry`       |
| SQS             | Queue inbound adapter| `approve_time_entry`      |
| EventBridge     | Event inbound adapter| react to external events  |
| SNS             | Event inbound adapter| broadcast event consumer  |
| DynamoDB Stream | Event inbound adapter| projection rebuilder      |

Each trigger translates its AWS-specific event payload into a domain command or query and calls the use case handler. The inbound adapter is responsible for that translation — the handler never knows which AWS service triggered it.

## Folder Structure

The shell grows a `lambdas/` subfolder alongside `local/`. Each Lambda entry point is a single file responsible for bootstrap and wiring only:

```
shell/
  lambdas/
    time_entries/
      create_time_entry.rs     // API Gateway → HTTP inbound adapter
      approve_time_entry.rs    // SQS → queue inbound adapter
      list_time_entries.rs     // API Gateway → HTTP inbound adapter
    approvals/
      request_approval.rs
      review_approval.rs
      list_approvals.rs
  local/
    main.rs                    // long-running dev server, mounts all routes

cdk/                           // outside src, infrastructure as code
  lib/
    time_entries_stack.ts
    approvals_stack.ts
  bin/
    app.ts
```

Use cases remain unchanged — no Lambda-specific code inside them:

```
use_cases/
  create_time_entry/
    mod.rs
    commands.rs
    decision.rs
    handler.rs
    inbound/
      http.rs                  // translates HTTP or API Gateway payload → command
      sqs.rs                   // translates SQS message → command (if needed)
```

## Lambda Bootstrap Pattern

Each Lambda entry point follows the same pattern:

```rust
// shell/lambdas/time_entries/create_time_entry.rs

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = TimeEntriesConfig::from_env();

    let event_store = DynamoDbEventStore::new(&config.database).await;
    let intent_outbox = SqsIntentOutbox::new(&config.kafka).await;
    let technical_store = CloudWatchTechnicalEventStore::new().await;

    let handler = CreateTimeEntryHandler::new(
        event_store,
        intent_outbox,
        technical_store.clone(),
    );

    let adapter = CreateTimeEntryHttpAdapter::new(handler, technical_store);

    lambda_runtime::run(service_fn(|event: LambdaEvent<ApiGatewayProxyRequest>| {
        adapter.handle(event)
    })).await
}
```

The Lambda entry point:
1. Reads config from environment variables (injected by CDK)
2. Instantiates concrete infrastructure implementations
3. Wires them into the use case handler
4. Wraps the inbound adapter in the Lambda runtime handler
5. Does nothing else

## CDK Stack Pattern

The CDK stack mirrors the use case structure — one Lambda function per use case, pointing at the compiled binary for that entry point:

```typescript
// cdk/lib/time_entries_stack.ts

const createTimeEntry = new RustFunction(this, 'CreateTimeEntry', {
  bin: 'create_time_entry',          // points to shell/lambdas/time_entries/create_time_entry.rs
  environment: {
    EVENT_STORE_TABLE: table.tableName,
    INTENT_OUTBOX_QUEUE: queue.queueUrl,
  },
});

const api = new RestApi(this, 'TimeEntriesApi');
api.root
  .addResource('time-entries')
  .addMethod('POST', new LambdaIntegration(createTimeEntry));

const approveTimeEntry = new RustFunction(this, 'ApproveTimeEntry', {
  bin: 'approve_time_entry',
  environment: {
    EVENT_STORE_TABLE: table.tableName,
    INTENT_OUTBOX_QUEUE: queue.queueUrl,
  },
});

const sqsTrigger = new SqsEventSource(commandQueue);
approveTimeEntry.addEventSource(sqsTrigger);
```

CDK knows about Lambda functions and AWS resources. It does not know about use case internals, domain types, or adapter implementations.

## Local Development

The `shell/local/main.rs` serves as a long-running Axum server that mounts all routes and registers all Kafka consumers. It uses in-memory infrastructure implementations instead of AWS ones. The same inbound adapters are reused — only the bootstrap and infrastructure implementations differ:

```rust
// shell/local/main.rs

let event_store = InMemoryEventStore::new();
let intent_outbox = InMemoryIntentOutbox::new();
let technical_store = InMemoryTechnicalEventStore::new();

// wire all use cases
let create_handler = CreateTimeEntryHandler::new(...);
let approve_handler = ApproveTimeEntryHandler::new(...);

// mount all routes
let app = Router::new()
    .merge(time_entries::routes(create_handler, approve_handler, ...))
    .merge(approvals::routes(...));

axum::serve(listener, app).await;
```

The same inbound adapters (`http.rs`, `sqs.rs`) work in both Lambda and local contexts because they depend on the handler through its port, not on the runtime.

## Relation to ADR-0001

This ADR extends ADR-0001 by specifying how the `shell/` layer adapts for a Lambda deployment target. The core structure defined in ADR-0001 — modules, use cases, inbound adapters, outbound adapters, shared infrastructure — is unchanged. Lambda is purely a deployment concern resolved entirely within `shell/lambdas/` and `cdk/`.
