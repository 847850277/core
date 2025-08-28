#!/bin/bash

# NEAR Smart Contract Deployment Script for Guess Number Game
# This script builds and deploys the guess number game contract to NEAR

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
CONTRACT_NAME="guess-number-game"
NETWORK="testnet"
BUILD_DIR="target/wasm32-unknown-unknown/release"
WASM_FILE="guess_number_contract.wasm"

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if required tools are installed
check_dependencies() {
    print_status "Checking dependencies..."

    if ! command -v near &> /dev/null; then
        print_error "NEAR CLI is not installed. Please install it first:"
        echo "npm install -g near-cli"
        exit 1
    fi

    if ! command -v rustc &> /dev/null; then
        print_error "Rust is not installed. Please install it first:"
        echo "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi

    # Check if wasm32 target is installed
    if ! rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
        print_warning "wasm32-unknown-unknown target not found. Installing..."
        rustup target add wasm32-unknown-unknown
    fi

    print_success "All dependencies are installed"
}

# Build the contract
build_contract() {
    print_status "Building the contract..."

    # Clean previous build
    cargo clean

    # Build the contract
    cargo build --target wasm32-unknown-unknown --release

    if [ ! -f "$BUILD_DIR/$WASM_FILE" ]; then
        print_error "Build failed! WASM file not found at $BUILD_DIR/$WASM_FILE"
        exit 1
    fi

    # Get file size
    SIZE=$(ls -lh "$BUILD_DIR/$WASM_FILE" | awk '{print $5}')
    print_success "Contract built successfully! Size: $SIZE"
}

# Check NEAR login status
check_near_login() {
    print_status "Checking NEAR login status..."

    if ! near list-keys --networkId $NETWORK &> /dev/null; then
        print_warning "You are not logged in to NEAR $NETWORK"
        print_status "Please login first:"
        echo "near login --networkId $NETWORK"
        read -p "Press enter after logging in..."
    fi

    print_success "NEAR login verified"
}

# Create account for contract (if needed)
create_contract_account() {
    local account_id="$1"

    print_status "Checking if contract account exists: $account_id"

    if near state "$account_id" --networkId $NETWORK &> /dev/null; then
        print_warning "Account $account_id already exists"
        read -p "Do you want to redeploy to existing account? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            print_status "Deployment cancelled"
            exit 0
        fi
    else
        print_status "Creating contract account: $account_id"

        # Get the current logged in account
        ACCOUNT=$(near list-keys --networkId $NETWORK 2>/dev/null | head -1 | cut -d' ' -f1 || echo "")

        if [ -z "$ACCOUNT" ]; then
            print_error "Could not determine logged in account"
            exit 1
        fi

        print_status "Creating subaccount $account_id from $ACCOUNT"

        # Create subaccount with 3 NEAR initial balance
        near create-account "$account_id" --masterAccount "$ACCOUNT" --initialBalance 3 --networkId $NETWORK

        if [ $? -eq 0 ]; then
            print_success "Contract account created: $account_id"
        else
            print_error "Failed to create contract account"
            exit 1
        fi
    fi
}

# Deploy the contract
deploy_contract() {
    local account_id="$1"

    print_status "Deploying contract to $account_id..."

    near deploy "$account_id" --wasmFile "$BUILD_DIR/$WASM_FILE" --networkId $NETWORK

    if [ $? -eq 0 ]; then
        print_success "Contract deployed successfully!"
    else
        print_error "Contract deployment failed"
        exit 1
    fi
}

# Initialize the contract
initialize_contract() {
    local account_id="$1"
    local owner_id="$2"

    print_status "Initializing contract with owner: $owner_id"

    near call "$account_id" new "{\"owner_id\": \"$owner_id\"}" --accountId "$owner_id" --networkId $NETWORK

    if [ $? -eq 0 ]; then
        print_success "Contract initialized successfully!"
    else
        print_error "Contract initialization failed"
        exit 1
    fi
}

# Test the contract with a sample game record
test_contract() {
    local account_id="$1"
    local player_id="$2"

    print_status "Testing contract with sample game record..."

    # Sample game record
    local game_record='{
        "game_id": "test_game_'$(date +%s)'",
        "player_id": "'$player_id'",
        "target_number": 42,
        "attempts": 3,
        "guesses": [25, 60, 42],
        "duration_seconds": 45,
        "timestamp": '$(date +%s)',
        "success": true,
        "difficulty": "normal",
        "score": 850
    }'

    print_status "Storing test game record..."
    near call "$account_id" store_game_record "$game_record" --accountId "$player_id" --deposit 0.01 --networkId $NETWORK

    if [ $? -eq 0 ]; then
        print_success "Test game record stored successfully!"

        # Get player stats
        print_status "Retrieving player stats..."
        near view "$account_id" get_player_stats "{\"player_id\": \"$player_id\"}" --networkId $NETWORK

        # Get contract stats
        print_status "Retrieving contract stats..."
        near view "$account_id" get_contract_stats --networkId $NETWORK
    else
        print_error "Test failed"
    fi
}

# Print helpful commands
print_usage_examples() {
    local account_id="$1"

    print_success "Deployment complete! Here are some useful commands:"
    echo
    echo -e "${YELLOW}View contract stats:${NC}"
    echo "near view $account_id get_contract_stats --networkId $NETWORK"
    echo
    echo -e "${YELLOW}Store a game record:${NC}"
    echo "near call $account_id store_game_record '{
  \"game_id\": \"unique_game_id\",
  \"player_id\": \"your_account.testnet\",
  \"target_number\": 42,
  \"attempts\": 5,
  \"guesses\": [50, 25, 60, 40, 42],
  \"duration_seconds\": 120,
  \"timestamp\": $(date +%s),
  \"success\": true,
  \"difficulty\": \"normal\",
  \"score\": 750
}' --accountId your_account.testnet --deposit 0.01 --networkId $NETWORK"
    echo
    echo -e "${YELLOW}Get player stats:${NC}"
    echo "near view $account_id get_player_stats '{\"player_id\": \"your_account.testnet\"}' --networkId $NETWORK"
    echo
    echo -e "${YELLOW}Get leaderboard:${NC}"
    echo "near view $account_id get_leaderboard '{\"limit\": 10}' --networkId $NETWORK"
    echo
    echo -e "${YELLOW}Get player game history:${NC}"
    echo "near view $account_id get_player_games '{\"player_id\": \"your_account.testnet\", \"limit\": 5}' --networkId $NETWORK"
    echo
    echo -e "${BLUE}Contract deployed at:${NC} https://explorer.testnet.near.org/accounts/$account_id"
}

# Main deployment function
main() {
    print_status "Starting NEAR Smart Contract deployment for Guess Number Game"
    echo "Network: $NETWORK"
    echo

    # Parse command line arguments
    ACCOUNT_ID=""
    OWNER_ID=""
    SKIP_BUILD=false
    RUN_TESTS=false

    while [[ $# -gt 0 ]]; do
        case $1 in
            --account)
                ACCOUNT_ID="$2"
                shift 2
                ;;
            --owner)
                OWNER_ID="$2"
                shift 2
                ;;
            --skip-build)
                SKIP_BUILD=true
                shift
                ;;
            --test)
                RUN_TESTS=true
                shift
                ;;
            --network)
                NETWORK="$2"
                shift 2
                ;;
            -h|--help)
                echo "Usage: $0 [OPTIONS]"
                echo "Options:"
                echo "  --account ACCOUNT_ID  Contract account ID (e.g., mycontract.testnet)"
                echo "  --owner OWNER_ID      Contract owner account ID"
                echo "  --skip-build          Skip building the contract"
                echo "  --test                Run tests after deployment"
                echo "  --network NETWORK     NEAR network (testnet/mainnet, default: testnet)"
                echo "  -h, --help            Show this help message"
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done

    # Get account ID if not provided
    if [ -z "$ACCOUNT_ID" ]; then
        read -p "Enter contract account ID (e.g., mycontract.testnet): " ACCOUNT_ID
        if [ -z "$ACCOUNT_ID" ]; then
            print_error "Account ID is required"
            exit 1
        fi
    fi

    # Get owner ID if not provided
    if [ -z "$OWNER_ID" ]; then
        # Try to get current logged in account
        OWNER_ID=$(near list-keys --networkId $NETWORK 2>/dev/null | head -1 | cut -d' ' -f1 || echo "")
        if [ -z "$OWNER_ID" ]; then
            read -p "Enter contract owner account ID: " OWNER_ID
            if [ -z "$OWNER_ID" ]; then
                print_error "Owner ID is required"
                exit 1
            fi
        else
            print_status "Using logged in account as owner: $OWNER_ID"
        fi
    fi

    # Run deployment steps
    check_dependencies

    if [ "$SKIP_BUILD" = false ]; then
        build_contract
    fi

    check_near_login
    create_contract_account "$ACCOUNT_ID"
    deploy_contract "$ACCOUNT_ID"
    initialize_contract "$ACCOUNT_ID" "$OWNER_ID"

    if [ "$RUN_TESTS" = true ]; then
        test_contract "$ACCOUNT_ID" "$OWNER_ID"
    fi

    print_usage_examples "$ACCOUNT_ID"

    print_success "Deployment completed successfully!"
}

# Run main function with all arguments
main "$@"
