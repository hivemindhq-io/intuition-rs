import { expect, test, suite } from 'vitest'
import { getIntuition, pinJson, SystemAtom } from './setup/utils.js'

test('create a list', async () => {
  const bob = await getIntuition(2)

  const hasTag = await bob.getOrCreateAtom(
    SystemAtom.Keywords,
  )

  await bob.getCreateOrDepositOnTriple(
    (await bob.getOrCreateAtom(
      'Metamask',
    )).vaultId,
    hasTag.vaultId,
    (await bob.getOrCreateAtom(
      'Wallet',
    )).vaultId,
  )
  await bob.getCreateOrDepositOnTriple(
    (await bob.getOrCreateAtom(
      'Rabby',
    )).vaultId,
    hasTag.vaultId,
    (await bob.getOrCreateAtom(
      'Wallet',
    )).vaultId,
  )
  await bob.getCreateOrDepositOnTriple(
    (await bob.getOrCreateAtom(
      'Coinbase',
    )).vaultId,
    hasTag.vaultId,
    (await bob.getOrCreateAtom(
      'Wallet',
    )).vaultId,
  )

  expect(hasTag).toBeDefined()

})

