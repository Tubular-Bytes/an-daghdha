
# Table of Contents

1.  [Introduction](#org4c75e65)
2.  [Main features](#org41d8603)
    1.  [Communication layer](#orgeebea71)
    2.  [Control layer](#org23f948c)
        1.  [Ticker](#org86b7aa5)
        2.  [Broker](#org3e9a5c8)
    3.  [Actor layer](#org7e15b3c)
    4.  [Persistence layer](#org2c2e628)
3.  [Database system](#org3f72f98)
4.  [Message bus](#org3c6fd3d)



<a id="org4c75e65"></a>

# Introduction

An Daghdha is my latest attempt to write a distributed backend for the game I&rsquo;ve been planning for years and rewrote many times.
The main focus is to create a fluid and easy to use framework for a macro-focused strategy game which is akin to 4x titles but takes much of the actual controls away from the players. It becomes more a resource-management game - dare say, a simulation - than a race for better APM or finding cheese tactics.


<a id="org41d8603"></a>

# Main features

The engine is written in Rust. It is a learning project - learning Rust itself, project management, self-improvement - as well as Rust being an optimal language when performance might be a focus down the line.
The engine consists of 4 layers. The communication layer, the control layer, the actor layer and the persistence layer.


<a id="orgeebea71"></a>

## Communication layer

The communication layer is the entrypoint to the application from the external world. It&rsquo;s a Websocket API that handles arbitrary messages sent from any client that has the knowledge to successfully authenticate with the service and call the API. At this point authentication is very simple, the client sends a handshake message with plaintext username and password to which the server replies with a token upon successful authentication.

For authorization the plan is to use PASETO tokens. PASETO is a standard that improves on JWT&rsquo;s shortcomings in several ways. If you&rsquo;re interested in PASETO and its advantages, head over to <https://paseto.io>

The communication layer has very few responsibilities. Apart from the aforementioned authz and authn duties it just validates and forwards messages to the control layer.


<a id="org23f948c"></a>

## Control layer

The control layer is where the main components of the engine live: the ticker - the heart - and the broker - the brain.


<a id="org86b7aa5"></a>

### Ticker

The ticker is responsible for broadcasting signals to actors - that reside in the actor layer - periodically. These signals can be used for anything: refresh cached data, persist changes or just in general the passage of time. It is up to the actors how they interpret these signals and what they do with them.


<a id="org3e9a5c8"></a>

### Broker

The broker is a smart proxy between the communication layer and the actor layer. Its main job is to receive tasks from the API and decide which actor to dispatch the task for, then listen or wait for the result and relay it back to the API.


<a id="org7e15b3c"></a>

## Actor layer

The name of this layer tells it all really, this is were different actors are created. Actors serve as independent state machines in the engine. Actors can&rsquo;t be mutated from external sources: they act on tasks they receive and they act on them in an encapsulated fashion. Actors can communicate with each other using the same message bus as they receive and send Broker messages from/to.


<a id="org2c2e628"></a>

## Persistence layer

This layer is also fairly simple. It deals with tasks related to persistence. In order for an actor to read or write the database it has to interact with this layer.


<a id="org3f72f98"></a>

# Database system

For database I chose PostgreSQL. It&rsquo;s a versatile tool for storing data and this versatility will come in handy if we ever get to the point where storing JSON becomes a bottleneck for queries and mutations.


<a id="org3c6fd3d"></a>

# Message bus

The message bus is a very broad term for all the channels that are used for communication and is probably the most complex part of the engine. It&rsquo;s used for communication between layers and actors and thus it&rsquo;s very important for it to be well-tested and performant.
There are several uses for the bus:

-   API <-> Broker - this is a two-way communication between the communication and the control layer. It should be a `broadcast` channel as we need the ability to both subscribe to a `Sender` and to clone one as well.
-   Broker to Actor - this is also a two-way communication, similar to the above. They might be good candidates to handle with the same data structure and loop.
    
    Both these structures need to be able to send instant replies for acknowledging or rejecting tasks even if the actual piece of work happens asynchronously.

