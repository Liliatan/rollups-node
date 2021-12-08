// Copyright 2021 Cartesi Pte. Ltd.

// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file except in compliance with the License. You may obtain a copy of the
// License at http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
// CONDITIONS OF ANY KIND, either express or implied. See the License for the
// specific language governing permissions and limitations under the License.

/// @title Specific ERC20 Portal interface
pragma solidity >=0.7.0;

interface ISERC20Portal {
    /// @notice deposit an amount of the specific ERC20 token in the portal contract and create tokens in L2
    /// @param _amount amount of the ERC20 token to be deposited
    /// @param _data information to be interpreted by L2
    /// @return hash of input generated by deposit
    function serc20Deposit(uint256 _amount, bytes calldata _data)
        external
        returns (bytes32);

    /// @notice withdraws an amount of the specific ERC20 from the Diamond
    /// @param _data data with withdrawal information
    /// @dev can only be called by the Diamond itself
    function serc20Withdrawal(bytes calldata _data) external returns (bool);

    // @notice emitted on ERC20 deposited
    event SERC20Deposited(address _sender, uint256 _amount, bytes _data);

    // @notice emitted on ERC20 withdrawal
    event SERC20Withdrawn(address payable _receiver, uint256 _amount);
}
