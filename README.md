# C2O3Fe2 (COFe): An Educational Command & Control Framework Prototype
## Deployment Guide
## Architectural Model Used

## Core Capabilities/Features and Implementation Details
MITRE ATT&CK Technique ID noted in parentheses where appropriate
1. 
2. 
3. 
4. 
## Testing Setup Used  
## Operational Limitations 
## TO-DO
### Refactoring
1. Separate Teamserver code into separate files for routes, implant routes, teamserver routes
2. Improve error handling by implementing IntoResponse instead
### Features
#### Top Priority
+ Implement custom header for implant UUID on beacon
+ Set up WebSockets for operator connection
+ Replace shared state HashMap with a database for persistent data storage
#### Opsec Related
Current top priority is to add authentication for operators and prevent implant spoofing
1. Authentication for operator CLI
2. Token authentication for implants and operator connections
    - Asymmetric cryptographic signing? 
    - JWTs?
    - Use mTLS? 
3. Add encryption/decryption for requests and responses (probably by adding middleware?) 
4. Add Redirectors to add resilience from IP blacklisting and to better conceal teamserver from defenders
    + Refer to https://github.com/bluscreenofjeff/Red-Team-Infrastructure-Wiki#https
#### Implant Effectiveness
1. Evasion Techniques to add (Linux)
    - Process masquerading / argument spoofing ([T1036.011](https://attack.mitre.org/techniques/T1036/011/))
    - Fileless execution using memfd_create ([T1620](https://attack.mitre.org/techniques/T1620/))
    - LD_PRELOAD Library Hijacking ([T1574.006](https://attack.mitre.org/techniques/T1574/006/))
    - Event Triggering using Udev ([T1546.017](https://attack.mitre.org/techniques/T1546/017/))
    - **Process Injection** ([T1055](https://attack.mitre.org/techniques/T1055/))
    - io_uring Process Subsystems
    - Kernel Modules (LKM), PAM Backdoors, and eBPF Bypasses
2. Setup additional channel for Websocket communication for implant
3. Complete Windows cross-compatibility
#### Teamserver functionality
1. File staging directory