# Docs - Discovery

Discovery allows for inter-app communication.

```mermaid
sequenceDiagram
	participant sub1 as Relay1.Sub
	participant relay1 as Relay1
	participant Server as Server
	participant relay2 as Relay2
	participant pub2 as Relay2.Pub
		Note left of sub1: 1. create publishers and subscribers
	relay1 -->> sub1: add_subscriber
	relay2 -->> pub2: add_publisher
	Note left of sub1: 2. register discovery
	relay1 -->> relay2: add_discovery_listener
	relay1 -->> relay2: discovery_target.send(sub.topic,ext_msg_send)
		Note left of sub1: 3. send external message
	pub2 -->> relay2: message
	relay2 -->> relay1: external_message_target.send()
	relay1 -->> sub1: message
```



## 1. Discovery

### Purpose

`relay2` always knows exactly what channels `relay1` has.

### Process
1. When a `discoveryTarget` is added to a relay, it is immediatly told about all existing `topicChannels`
	- The target relay processes the messagags on `process_discovery_messages`.
	- At this stage it adds the `ExternalMessageTarget`
2. when the first publisher or subscriber belonging to `relay1` is added (when the `topicChannels` created), everybody in its `discoveryTargets` is notified via `relay.send_discovery(DiscoveryMessage)`.



## 2. Exernal Messaging

### Purpose
When a publisher belonging to `relay1` publishes a message, it is received by a subscriber on `relay2`.
### Process

1. every `Publisher`, after its `send.broadcast()`, will notify its relay via `relay.send_external_message((topic,message))`.
2. The parent relay broadcasts it to each other relay in its `external_targets: HashMap<TypedTopic,Vec<ExternalTarget>>` vec.
3. The other relay receives the  message via `relay.handle_external_message((Topic,Message))`and broadcasts it via `topic_channels[topic].send(Message)` if it has any for that topic.

```pseudo
relay1.pub1.on_publish(|message| relay2.sub1.recv(message))
```

## 3. Boundaries


### Discovery over boundaries
```mermaid
sequenceDiagram
	participant relay1 as Relay1
	participant client1 as Boundary1
	participant client2 as Boundary2
	participant relay2 as Relay2
	Note right of relay2: 1. let me know what topics you're interested in
	relay2 -->> client2: discovery_sender
	client2 -->> client1: local_relay_id
	client1 -->> relay1: remote_discovery_sender
	Note left of relay1: 2. Ok, I'm interested in this topic
	relay1 -->> client1: external_send
	client1 -->> client2: remote_relay_id
	client2 -->> relay2: remote_external_send





```




## 4. Propagation

```mermaid
graph LR;
	subgraph App One
	R1 --> B12
	end
	B12 --> B21

	subgraph App Two
	B21 --> R2
	R2 --> B23
	R2 --> B24
	end	
	
	B23 --> B32
	
	subgraph App Four
	B42 --> R4
	end

	subgraph App Three
	B32 --> R3
	end
	B24 --> B42

R1[Relay 1]
R2[Relay 2]
R3[Relay 3]
R4[Relay 4]
B12[Boundary 1-2 \n Remote 2]
B21[Boundary 2-1]
B23[Boundary 2-3]
B32[Boundary 3-2]
B24[Boundary 2-4]
B42[Boundary 4-2]

```

- TODO this is wrong, we need remote proxies, and seperate from that a sync.


### Purpose

Consider the connection of `App 1 <-> App 2 <-> App 3`. We



### Process

// TODO
