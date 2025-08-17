
# Table of Contents

1.  [Message bus planning](#org74835c2)
    1.  [Application -> All](#org7eea1e9)
    2.  [API -> Actors](#org3d6a36f)
    3.  [Ticker -> Actors](#orgf49a130)
2.  [Actor types](#orgd42b0a5)
    1.  [Authentication and authorization](#org4fba9f7)
    2.  [Storage](#org38202a3)
    3.  [Inventory](#org402acfc)


<a id="org74835c2"></a>

# Message bus planning

what if the broker IS the bus? no logic, just relaying messages.

topics are logical separation between messages, it uses the same broadcast channel, topic is encapsulated in the message, actors decide what they process.


<a id="org7eea1e9"></a>

## Application -> All

shutdown


<a id="org3d6a36f"></a>

## API -> Actors

auth, task topic


<a id="orgf49a130"></a>

## Ticker -> Actors

tick topic


<a id="orgd42b0a5"></a>

# Actor types

Authx - responsible for authorization and authentication - direct database access
Storage - responsible for distributing IO for the database - direct database access
Inventory - represents a single account, keeps track of assets - database access through Storage actor


<a id="org4fba9f7"></a>

## Authentication and authorization

-   Authn: API receives an authn message, dispatches a task to the auth actor and waits for the answer or timeout.
    Task should include username, password and the reply channel.
-   Authz: API receives any operations other than authn with an `Authorization` header, dispatches a task with the token for validation and waits for the answer or timeout.
    Task should include PASETO token and the reply channel.


<a id="org38202a3"></a>

## Storage

Inventory actors are responsible for persisting and caching data but these operations happen through the Storage actor. This actor listens for the usual CRUD operations and perform the tasks asynchronously. They emit two replies: one is instant, acknowledging the operation or returning possible validation errors, the other is sent via the reply channel if it&rsquo;s included in the task. This reply is optional for most operations except `read`: this operation requires the reply channel to send the requested value.


<a id="org402acfc"></a>

## Inventory

These actors manipulate the database via the storage layer to indicate state changes for assets they manage: resources, buildings, etc&#x2026;

