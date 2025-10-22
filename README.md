## Env
iroh 0.94.0

iroh-doc 0.94.0

## Usage Examples

### Starting the Service
```bash
./iroh-test --secret-key "[40, 151, 89, 230, 36, 193, 240, 70, 230, 182, 91, 52, 90, 153, 54, 56, 6, 119, 150, 167, 205, 214, 35, 40, 130, 88, 92, 231, 120, 46, 148, 46]" server
```
It will automatically populate test data after startup.

### Post-Startup Interaction
After the program starts, it will display:
```
Waiting for input or Ctrl+C...
```

At this point, you can input the following commands:
- Enter `add` to add image data
- Enter `add_folder` to add folder data
- Enter `get` to view the number of image data entries
- Enter `get_folder` to view the number of folder data entries

### How to Join the Service

After startup, the following will be generated:
``` bash
./iroh-test --secret-key "[89,188,181,9,112,70,251,252,214,80,117,4,225,245,67,162,60,124,215,26,121,9, 14, 212, 25, 38, 103, 185, 247, 133, 224, 240]" client docaaacbkusdbzrur7nyolncrrqo7urfaeo36gknsqioh3mc7yo3glidwb3ag7hfes3xkzqtqy.....
```

Switch to a different network domain and use this command to join the network.

Before running, please clear the cache data in the runtime directories: ./client and ./server.

## How to Test
edit main.rs --- 182 lines
