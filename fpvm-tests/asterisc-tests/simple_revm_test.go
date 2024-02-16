package asterisc_test

import (
	"crypto/sha256"
	"debug/elf"
	"strings"
	"testing"

	"github.com/ethereum/go-ethereum/common"
	"github.com/stretchr/testify/require"

	"github.com/ethereum-optimism/asterisc/rvgo/fast"
	preimage "github.com/ethereum-optimism/optimism/op-preimage"
)

func rustTestOracle(t *testing.T) (po PreimageOracle, stdOut string, stdErr string) {
	images := make(map[[32]byte][]byte)
	sha2Preimages := make(map[[32]byte][]byte)

	input := []byte("facade facade facade")
	shaHash := sha256.Sum256(input)
	images[preimage.LocalIndexKey(1).PreimageKey()] = shaHash[:]
	sha2Preimages[shaHash] = input

	// CALLDATASIZE
	// PUSH0
	// PUSH0
	// CALLDATACOPY
	// CALLDATASIZE
	// PUSH0
	// RETURN
	images[preimage.LocalIndexKey(2).PreimageKey()] = common.Hex2Bytes("365f5f37365ff3")

	oracle := &testOracle{
		hint: func(v []byte) {
			hintStr := string(v)
			hintParts := strings.Split(hintStr, " ")

			switch hintParts[0] {
			case "sha2-preimage":
				hash := common.HexToHash(hintParts[1])
				images[preimage.LocalIndexKey(0).PreimageKey()] = sha2Preimages[hash]
			default:
				t.Fatalf("unknown hint: %s", hintStr)
			}
		},
		getPreimage: func(k [32]byte) []byte {
			p, ok := images[k]
			if !ok {
				t.Fatalf("missing pre-image %s", k)
			}
			return p
		},
	}

	return oracle, "", ""
}

func TestSimpleRevm(t *testing.T) {
	programELF, err := elf.Open("../../examples/simple-revm/target/riscv64gc-unknown-none-elf/release/simple-revm")
	require.NoError(t, err)
	defer programELF.Close()

	po, _, _ := rustTestOracle(t)

	symbols, err := fast.Symbols(programELF)
	require.NoError(t, err)

	vmState, err := fast.LoadELF(programELF)
	require.NoError(t, err, "must load test suite ELF binary")

	stdOut, stdErr := fullTest(t, vmState, po, symbols)
	require.Equal(t, stdOut.String(), "Booting EVM and checking hash...\nSuccess, hashes matched!\n")
	require.Equal(t, stdErr.String(), "")
}
