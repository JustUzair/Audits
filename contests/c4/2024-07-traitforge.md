# [H-1] Contracts inheriting the Pausable contract cannot pause or unpause functionality in case of emergency

## Impact

`DevFund.sol`, `EntityForging.sol`, `EntityTrading.sol`, `EntropyGenerator.sol`, `NukeFund.sol` these contracts inherit the `Pausable.sol` contract used for pausing and unpausing the functionality, generally in cases of emergency. Although the contract inherits `Pausable.sol`, it lacks necessary public/external functions that can be called by owner/admin to pause and unpause the contracts.

Refer to impacted line(s) of code here:

- [DevFund.sol#L9](https://github.com/code-423n4/2024-07-traitforge/blob/279b2887e3d38bc219a05d332cbcb0655b2dc644/contracts/DevFund/DevFund.sol#L9)

- [EntityForging.sol#L10](https://github.com/code-423n4/2024-07-traitforge/blob/279b2887e3d38bc219a05d332cbcb0655b2dc644/contracts/EntityForging/EntityForging.sol#L10)

- [EntityTrading.sol#L11](https://github.com/code-423n4/2024-07-traitforge/blob/279b2887e3d38bc219a05d332cbcb0655b2dc644/contracts/EntityTrading/EntityTrading.sol#L11)

- [EntropyGenerator.sol#L9](https://github.com/code-423n4/2024-07-traitforge/blob/279b2887e3d38bc219a05d332cbcb0655b2dc644/contracts/EntropyGenerator/EntropyGenerator.sol#L9)

- [NukeFund.sol#L11](https://github.com/code-423n4/2024-07-traitforge/blob/279b2887e3d38bc219a05d332cbcb0655b2dc644/contracts/NukeFund/NukeFund.sol#L11)

- [TraitForgeNft.sol#L14-L19](https://github.com/code-423n4/2024-07-traitforge/blob/main/contracts/TraitForgeNft/TraitForgeNft.sol#L14-L19)

## Tools Used

Manual Review

## Recommended Mitigation Steps

- Add public functions in all the contracts that inherit `Pausable.sol` to allow pausing and unpausing by owner in case of emergency

```diff
+    function pause() public onlyOwner {
+        _pause();
+    }

+    function unpause() public onlyOwner {
+        _unpause();
+    }
```

---

# [M-1] NFTs can be minted beyond more than maximum generation due to missing validation in `TraitForgeNft.sol::_mintInternal()`

## Impact

The `TraitForgeNft.sol::_mintInternal()` function used by internally by `mintToken()` and `mintWithBudget()` lacks input validation for maximum generation of NFTs allowing users to mint an NFT which can have `generation > 10`. The intended functionality is that each generation has 10,000 NFTs as depicted by `maxTokensPerGen` variable, and the maximum generations can be 10 depicted by `maxGeneration` variable.

Refer to impacted line(s) of code here:

- [TraitForgeNft.sol#L280-L309](https://github.com/code-423n4/2024-07-traitforge/blob/main/contracts/TraitForgeNft/TraitForgeNft.sol#L280-L309)

## Tools Used

Manual Review

## Recommended Mitigation Steps

- Add a check to ensure that the `currentGeneration`, after incrementing the generation count, does not exceed the `maxGeneration` limit.

```diff
    function _mintInternal(address to, uint256 mintPrice) internal {
      if (generationMintCounts[currentGeneration] >= maxTokensPerGen) {
        _incrementGeneration();
      }
      // @audit - missing max generation check, max generation is 10, does not account for it
      // @fix - add a check for maxGeneration
+     require(currentGeneration <= maxGeneration, "Generation count exceeded max count");
      _tokenIds++;
      uint256 newItemId = _tokenIds;
      _mint(to, newItemId);
      uint256 entropyValue = entropyGenerator.getNextEntropy();

      tokenCreationTimestamps[newItemId] = block.timestamp;
      tokenEntropy[newItemId] = entropyValue;
      tokenGenerations[newItemId] = currentGeneration;
      generationMintCounts[currentGeneration]++;
      initialOwners[newItemId] = to;

      if (!airdropContract.airdropStarted()) {
        airdropContract.addUserAmount(to, entropyValue);
      }

      emit Minted(
        msg.sender,
        newItemId,
        currentGeneration,
        entropyValue,
        mintPrice
      );

      _distributeFunds(mintPrice);
  }
```
