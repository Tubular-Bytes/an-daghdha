
# Table of Contents

1.  [Message bus planning](#org67069e1)
2.  [Actor types](#orgca037f6)
    1.  [Authentication and authorization](#orgc84fca6)
    2.  [Storage](#org9e94326)
    3.  [Inventory](#org15bf85a)


<a id="org67069e1"></a>

# Message bus planning

Topics are logical separation between messages, it uses the same broadcast channel, topic is encapsulated in the message, actors decide what they process.
This way actors can listen to topics by a wildcard and then use match statements to filter the selected tasks.

Message types depending on communication participants:

-   Application -> All
    shutdown
-   API -> Actors
    auth, task
-   Ticker -> Actors
    tick


<a id="orgca037f6"></a>

# Actor types

Authx - responsible for authorization and authentication - direct database access
Storage - responsible for distributing IO for the database - direct database access
Inventory - represents a single account, keeps track of assets - database access through Storage actor


<a id="orgc84fca6"></a>

## Authentication and authorization

-   Authn: API receives an authn message, dispatches a task to the auth actor and waits for the answer or timeout.
    Task should include username, password and the reply channel.
-   Authz: API receives any operations other than authn with an `Authorization` header, dispatches a task with the token for validation and waits for the answer or timeout.
    Task should include PASETO token and the reply channel.


<a id="org9e94326"></a>

## Storage

Inventory actors are responsible for persisting and caching data but these operations happen through the Storage actor. This actor listens for the usual CRUD operations and perform the tasks asynchronously. They emit two replies: one is instant, acknowledging the operation or returning possible validation errors, the other is sent via the reply channel if it&rsquo;s included in the task. This reply is optional for most operations except `read`: this operation requires the reply channel to send the requested value.


<a id="org15bf85a"></a>

## Inventory

These actors manipulate the database via the storage layer to indicate state changes for assets they manage: resources, buildings, etc&#x2026;

