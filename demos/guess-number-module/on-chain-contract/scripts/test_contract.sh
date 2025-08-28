#!/bin/bash

# NEAR Smart Contract Test Script for Guess Number Game
# This script tests all major functionality of the deployed contract

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
NETWORK="testnet"
CONTRACT_ACCOUNT=""
TEST_ACCOUNT=""
OWNER_ACCOUNT=""

# Test counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

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

# Test result tracking
test_start() {
    local test_name="$1"
    TESTS_RUN=$((TESTS_RUN + 1))
    print_status "Running test: $test_name"
}

test_pass() {
    local test_name="$1"
    TESTS_PASSED=$((TESTS_PASSED + 1))
    print_success "✓ $test_name"
}

test_fail() {
    local test_name="$1"
    local error="$2"
    TESTS_FAILED=$((TESTS_FAILED + 1))
    print_error "✗ $test_name: $error"
}

# Generate unique test data
generate_game_id() {
    echo "test_game_$(date +%s)_$RANDOM"
}

generate_timestamp() {
    date +%s
}

# Wait for transaction to be processed
wait_for_tx() {
    sleep 2
}

# Test contract initialization
test_contract_init() {
    test_start "Contract initialization"

    local result
    if result=$(near view "$CONTRACT_ACCOUNT" get_contract_stats --networkId "$NETWORK" 2>&1); then
        if echo "$result" | grep -q "total_games"; then
            test_pass "Contract initialization"
        else
            test_fail "Contract initialization" "Invalid contract stats format"
        fi
    else
        test_fail "Contract initialization" "Contract not properly initialized"
    fi
}

# Test storing a game record
test_store_game_record() {
    test_start "Store game record"

    local game_id
    game_id=$(generate_game_id)
    local timestamp
    timestamp=$(generate_timestamp)

    local game_record='{
        "game_id": "'$game_id'",
        "player_id": "'$TEST_ACCOUNT'",
        "target_number": 42,
        "attempts": 3,
        "guesses": [25, 60, 42],
        "duration_seconds": 45,
        "timestamp": '$timestamp',
        "success": true,
        "difficulty": "normal",
        "score": 850
    }'

    if near call "$CONTRACT_ACCOUNT" store_game_record "$game_record" --accountId "$TEST_ACCOUNT" --deposit 0.01 --networkId "$NETWORK" >/dev/null 2>&1; then
        wait_for_tx

        # Verify the record was stored
        local stored_record
        if stored_record=$(near view "$CONTRACT_ACCOUNT" get_game_record '{"game_id": "'$game_id'"}' --networkId "$NETWORK" 2>&1); then
            if echo "$stored_record" | grep -q "$game_id"; then
                test_pass "Store game record"
                echo "$game_id" # Return game_id for other tests
            else
                test_fail "Store game record" "Record not found after storage"
            fi
        else
            test_fail "Store game record" "Failed to retrieve stored record"
        fi
    else
        test_fail "Store game record" "Failed to store game record"
    fi
}

# Test getting player stats
test_get_player_stats() {
    test_start "Get player stats"

    local stats
    if stats=$(near view "$CONTRACT_ACCOUNT" get_player_stats '{"player_id": "'$TEST_ACCOUNT'"}' --networkId "$NETWORK" 2>&1); then
        if echo "$stats" | grep -q "total_games" && echo "$stats" | grep -q "win_rate"; then
            test_pass "Get player stats"

            # Print stats for verification
            print_status "Player stats: $(echo "$stats" | tr -d '\n')"
        else
            test_fail "Get player stats" "Invalid stats format"
        fi
    else
        test_fail "Get player stats" "Failed to get player stats"
    fi
}

# Test getting player game history
test_get_player_games() {
    test_start "Get player games"

    local games
    if games=$(near view "$CONTRACT_ACCOUNT" get_player_games '{"player_id": "'$TEST_ACCOUNT'", "from_index": 0, "limit": 5}' --networkId "$NETWORK" 2>&1); then
        if echo "$games" | grep -q "\["; then
            test_pass "Get player games"

            # Count games
            local game_count
            game_count=$(echo "$games" | grep -o "game_id" | wc -l)
            print_status "Found $game_count games in history"
        else
            test_fail "Get player games" "Invalid games format"
        fi
    else
        test_fail "Get player games" "Failed to get player games"
    fi
}

# Test getting leaderboard
test_get_leaderboard() {
    test_start "Get leaderboard"

    local leaderboard
    if leaderboard=$(near view "$CONTRACT_ACCOUNT" get_leaderboard '{"limit": 5}' --networkId "$NETWORK" 2>&1); then
        if echo "$leaderboard" | grep -q "\["; then
            test_pass "Get leaderboard"

            # Count entries
            local entry_count
            entry_count=$(echo "$leaderboard" | grep -o "rank" | wc -l)
            print_status "Found $entry_count entries in leaderboard"
        else
            test_fail "Get leaderboard" "Invalid leaderboard format"
        fi
    else
        test_fail "Get leaderboard" "Failed to get leaderboard"
    fi
}

# Test search players
test_search_players() {
    test_start "Search players"

    local search_term
    search_term=$(echo "$TEST_ACCOUNT" | cut -c1-5)

    local results
    if results=$(near view "$CONTRACT_ACCOUNT" search_players '{"query": "'$search_term'"}' --networkId "$NETWORK" 2>&1); then
        if echo "$results" | grep -q "\["; then
            test_pass "Search players"
        else
            test_fail "Search players" "Invalid search results format"
        fi
    else
        test_fail "Search players" "Failed to search players"
    fi
}

# Test getting recent games
test_get_recent_games() {
    test_start "Get recent games"

    local recent_games
    if recent_games=$(near view "$CONTRACT_ACCOUNT" get_recent_games '{"limit": 10}' --networkId "$NETWORK" 2>&1); then
        if echo "$recent_games" | grep -q "\["; then
            test_pass "Get recent games"
        else
            test_fail "Get recent games" "Invalid recent games format"
        fi
    else
        test_fail "Get recent games" "Failed to get recent games"
    fi
}

# Test contract stats
test_contract_stats() {
    test_start "Get contract stats"

    local stats
    if stats=$(near view "$CONTRACT_ACCOUNT" get_contract_stats --networkId "$NETWORK" 2>&1); then
        if echo "$stats" | grep -q "total_games" && echo "$stats" | grep -q "version"; then
            test_pass "Get contract stats"
            print_status "Contract stats: $(echo "$stats" | tr -d '\n')"
        else
            test_fail "Get contract stats" "Invalid contract stats format"
        fi
    else
        test_fail "Get contract stats" "Failed to get contract stats"
    fi
}

# Test invalid game record (should fail)
test_invalid_game_record() {
    test_start "Invalid game record (should fail)"

    local game_id
    game_id=$(generate_game_id)

    local invalid_record='{
        "game_id": "'$game_id'",
        "player_id": "'$TEST_ACCOUNT'",
        "target_number": 42,
        "attempts": 3,
        "guesses": [25, 60],
        "duration_seconds": 45,
        "timestamp": '$(generate_timestamp)',
        "success": true,
        "difficulty": "invalid_difficulty",
        "score": 850
    }'

    if near call "$CONTRACT_ACCOUNT" store_game_record "$invalid_record" --accountId "$TEST_ACCOUNT" --deposit 0.01 --networkId "$NETWORK" >/dev/null 2>&1; then
        test_fail "Invalid game record (should fail)" "Invalid record was accepted"
    else
        test_pass "Invalid game record (should fail)"
    fi
}

# Test duplicate game ID (should fail)
test_duplicate_game_id() {
    test_start "Duplicate game ID (should fail)"

    # First, store a valid record
    local game_id
    game_id=$(generate_game_id)
    local timestamp
    timestamp=$(generate_timestamp)

    local game_record='{
        "game_id": "'$game_id'",
        "player_id": "'$TEST_ACCOUNT'",
        "target_number": 50,
        "attempts": 4,
        "guesses": [25, 75, 60, 50],
        "duration_seconds": 60,
        "timestamp": '$timestamp',
        "success": true,
        "difficulty": "easy",
        "score": 700
    }'

    # Store first record
    if near call "$CONTRACT_ACCOUNT" store_game_record "$game_record" --accountId "$TEST_ACCOUNT" --deposit 0.01 --networkId "$NETWORK" >/dev/null 2>&1; then
        wait_for_tx

        # Try to store duplicate
        if near call "$CONTRACT_ACCOUNT" store_game_record "$game_record" --accountId "$TEST_ACCOUNT" --deposit 0.01 --networkId "$NETWORK" >/dev/null 2>&1; then
            test_fail "Duplicate game ID (should fail)" "Duplicate game ID was accepted"
        else
            test_pass "Duplicate game ID (should fail)"
        fi
    else
        test_fail "Duplicate game ID (should fail)" "Failed to store initial record"
    fi
}

# Test admin functions (if owner)
test_admin_functions() {
    if [ "$TEST_ACCOUNT" = "$OWNER_ACCOUNT" ]; then
        test_start "Admin functions"

        # Test rebuild leaderboard
        if near call "$CONTRACT_ACCOUNT" rebuild_leaderboard_admin --accountId "$OWNER_ACCOUNT" --networkId "$NETWORK" >/dev/null 2>&1; then
            test_pass "Admin functions"
        else
            test_fail "Admin functions" "Failed to rebuild leaderboard"
        fi
    else
        print_status "Skipping admin function tests (not owner)"
    fi
}

# Load test - store multiple game records
test_load_multiple_records() {
    test_start "Load test - multiple records"

    local success_count=0
    local total_records=5

    for i in $(seq 1 $total_records); do
        local game_id
        game_id="load_test_$(generate_game_id)"
        local target=$((RANDOM % 100 + 1))
        local attempts=$((RANDOM % 10 + 1))
        local success=$([[ $((RANDOM % 2)) -eq 0 ]] && echo "true" || echo "false")

        local game_record='{
            "game_id": "'$game_id'",
            "player_id": "'$TEST_ACCOUNT'",
            "target_number": '$target',
            "attempts": '$attempts',
            "guesses": [50, 75, 25],
            "duration_seconds": 30,
            "timestamp": '$(generate_timestamp)',
            "success": '$success',
            "difficulty": "normal",
            "score": 500
        }'

        if near call "$CONTRACT_ACCOUNT" store_game_record "$game_record" --accountId "$TEST_ACCOUNT" --deposit 0.01 --networkId "$NETWORK" >/dev/null 2>&1; then
            success_count=$((success_count + 1))
        fi

        wait_for_tx
    done

    if [ "$success_count" -eq "$total_records" ]; then
        test_pass "Load test - multiple records"
        print_status "Successfully stored $success_count/$total_records records"
    else
        test_fail "Load test - multiple records" "Only $success_count/$total_records records stored"
    fi
}

# Print test results
print_test_results() {
    echo
    echo "=================================="
    echo "           TEST RESULTS           "
    echo "=================================="
    echo "Total tests run: $TESTS_RUN"
    print_success "Tests passed: $TESTS_PASSED"
    if [ "$TESTS_FAILED" -gt 0 ]; then
        print_error "Tests failed: $TESTS_FAILED"
    else
        print_success "Tests failed: $TESTS_FAILED"
    fi
    echo "=================================="
    echo

    if [ "$TESTS_FAILED" -eq 0 ]; then
        print_success "🎉 All tests passed!"
        exit 0
    else
        print_error "❌ Some tests failed"
        exit 1
    fi
}

# Main function
main() {
    print_status "Starting NEAR Smart Contract Tests"
    echo

    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --contract)
                CONTRACT_ACCOUNT="$2"
                shift 2
                ;;
            --test-account)
                TEST_ACCOUNT="$2"
                shift 2
                ;;
            --owner)
                OWNER_ACCOUNT="$2"
                shift 2
                ;;
            --network)
                NETWORK="$2"
                shift 2
                ;;
            -h|--help)
                echo "Usage: $0 [OPTIONS]"
                echo "Options:"
                echo "  --contract ACCOUNT      Contract account ID"
                echo "  --test-account ACCOUNT  Test account ID"
                echo "  --owner ACCOUNT         Owner account ID"
                echo "  --network NETWORK       NEAR network (default: testnet)"
                echo "  -h, --help              Show this help message"
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done

    # Validate required parameters
    if [ -z "$CONTRACT_ACCOUNT" ]; then
        read -p "Enter contract account ID: " CONTRACT_ACCOUNT
    fi

    if [ -z "$TEST_ACCOUNT" ]; then
        read -p "Enter test account ID: " TEST_ACCOUNT
    fi

    if [ -z "$OWNER_ACCOUNT" ]; then
        OWNER_ACCOUNT="$TEST_ACCOUNT"
    fi

    if [ -z "$CONTRACT_ACCOUNT" ] || [ -z "$TEST_ACCOUNT" ]; then
        print_error "Contract account and test account are required"
        exit 1
    fi

    print_status "Configuration:"
    echo "  Network: $NETWORK"
    echo "  Contract: $CONTRACT_ACCOUNT"
    echo "  Test Account: $TEST_ACCOUNT"
    echo "  Owner Account: $OWNER_ACCOUNT"
    echo

    # Check if accounts exist
    print_status "Verifying accounts..."

    if ! near state "$CONTRACT_ACCOUNT" --networkId "$NETWORK" >/dev/null 2>&1; then
        print_error "Contract account does not exist: $CONTRACT_ACCOUNT"
        exit 1
    fi

    if ! near state "$TEST_ACCOUNT" --networkId "$NETWORK" >/dev/null 2>&1; then
        print_error "Test account does not exist: $TEST_ACCOUNT"
        exit 1
    fi

    print_success "All accounts verified"
    echo

    # Run tests
    print_status "Starting test execution..."
    echo

    # Basic functionality tests
    test_contract_init
    test_store_game_record
    test_get_player_stats
    test_get_player_games
    test_get_leaderboard
    test_search_players
    test_get_recent_games
    test_contract_stats

    # Error handling tests
    test_invalid_game_record
    test_duplicate_game_id

    # Admin tests
    test_admin_functions

    # Load tests
    test_load_multiple_records

    # Print final results
    print_test_results
}

# Run main function with all arguments
main "$@"
