### [H-1] Password stored on-chain makes it visible to anyone, and no longer private

**Description:** All data stored on-chain is visible to anyone, and can be read directly from the blockchain. The `PasswordStore::s_password` variable is intended to be a private variable and only accessed through the `PasswordStore::getPassword()` function, which is intended to be only called by the owner of the contract.

We show one such method of reading any data off chain below.

**Impact:** Anyone can read the password, severly breaking the functionality of the protocol

**Proof of Concept:** (Proof of Code)
The below test case shows how anyone can read the password directly from the blockchain

1. Create a locally running chain

```bash
make anvil
```

2. Deploy contract on chain

```bash
make deploy
```

3. Run the storage tool

```bash
cast storage <DEPLOYED_CONTRACT_ADDRESS> 1 --rpc-url http://localhost:8545
```

You get output as such:
`0x6d7950617373776f726400000000000000000000000000000000000000000014`

4. Convert the output to a readable string

```bash
 cast parse-bytes32-string 0x6d7950617373776f726400000000000000000000000000000000000000000014
```

You get output as such:
`myPassword`

**Recommended Mitigation:**

1. encrypt the password off chain and then store encrypted password on chain
2. User would reqiure to remmeber another password for decrpytion of encrypted password
3. Remove view function, as you wouldn't want the user to accidentally send a transaction with the password that decrypts your password

### [H-2] `PasswordStore::setPassword()` has no access controls, meaning a non-owner could change the password

**Description:** `PasswordStore::setPassword()` function is set to be an external function, the natspec of the function and overall purpose of the smart contract is that `The function allows only owner to set a new password`

```javascript
function setPassword(string memory newPassword) external {
@>      // @audit - missing access control
        s_password = newPassword;
        emit SetNetPassword();
}
```

**Impact:**
Anyone can set the password of the contract breaking the functionality of the contract.

**Proof of Concept:**
Add the following to `PasswordStore.t.sol` test file:

<details>
<summary>Code</summary>

```javascript
function test_anyone_can_set_password(address randomAddress) public {
        vm.assume(randomAddress != owner);
        vm.prank(randomAddress);
        string memory expectedPassword = "myNewPassword";
        passwordStore.setPassword(expectedPassword);
        vm.prank(owner);
        string memory actualPassword = passwordStore.getPassword();
        assertEq(actualPassword, expectedPassword);
    }
```

</details>

**Recommended Mitigation:** Add an access control conditional to `setPassword()` function.

```javascript
if(msg.sender != owner) {
    revert PasswordStore__NotOwner();
}
```

### [I-1] `PasswordStore::getPassword()` natspec indicates a parameter that doesn't exist, causing natspec to be incorrect

**Description:**
`PasswordStore::getPassword()` natspec indicates signature `PasswordStore::getPassword(string)` while actual code indicates `PasswordStore::getPassword()`

```javascript
 @param newPassword The new password to set.
```

**Impact:**
The natspec is incorrect

**Recommended Mitigation:**
Remove incorrect natspec

```diff
- *  @param newPassword The new password to set.
```
