# Incognichat

Incognichat is a secure and anonymous chat application designed for private, identity-free communication. Whether you want to have candid conversations or exchange messages without revealing your identity, Incognichat ensures your privacy and security.

## Features

- **Anonymity**: No personal data or registration required.
- **Secure Communication**: Messages are encrypted to protect your privacy.
- **Ephemeral Chats**: Option for self-destructing messages.
- **Cross-Platform Support**: Accessible on all platforms (linux, mac, windows).
- **Real-Time Messaging**: Instant delivery of messages with low latency.

## How It Works

1. **Create a Chat Room**: Generate a unique room ID to share with participants.
2. **Join a Room**: Use the room ID to connect anonymously.
3. **Start Chatting**: Exchange messages without revealing your identity.
4. **Leave No Trace**: Messages and connections are securely discarded after the session.

### Architecture Details
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Main Task     │    │ Stream Handler  │    │ Input Handler   │
│   (UI + Logic)  │    │   (Network)     │    │   (Keyboard)    │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                       ┌─────────────────┐
                       │  Event Channel  │
                       │   (Unbounded)   │
                       └─────────────────┘


Keyboard ──┐
           │
           ▼
    ┌─────────────┐      ┌─────────────┐      ┌─────────────┐
    │Input Handler│────▶ │   Channel   │────▶ │ Main Task   │
    └─────────────┘      └─────────────┘      └─────────────┘
                                ▲                     │
                                │                     │
    ┌──────────────┐            │                     ▼
    │Stream Handler│────────────┘              ┌─────────────┐
    └──────────────┘                           │   Render    │
           ▲                                   └─────────────┘
           │                                         │
    Network Socket                                   ▼
                                              Terminal Screen
## Use Cases

- Anonymous discussions.
- Temporary collaboration.
- Privacy-focused communication.

## Contribution

Contributions are welcome! Feel free to submit issues or pull requests to help improve Incognichat.
