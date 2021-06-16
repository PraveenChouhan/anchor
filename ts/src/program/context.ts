import {
  AccountMeta,
  Signer,
  ConfirmOptions,
  TransactionInstruction,
} from "@solana/web3.js";
import { Address } from "./common";
import { IdlAccountItem, IdlAccounts, IdlInstruction } from "../idl";

/**
 * Context provides all non-argument inputs for generating Anchor transactions.
 */
export type Context<A extends Accounts = Accounts> = {
  /**
   * Accounts used in the instruction context.
   */
  accounts?: A;

  /**
   * All accounts to pass into an instruction *after* the main `accounts`.
   * This can be used for optional or otherwise unknown accounts.
   */
  remainingAccounts?: AccountMeta[];

  /**
   * Accounts that must sign a given transaction.
   */
  signers?: Array<Signer>;

  /**
   * Instructions to run *before* a given method. Often this is used, for
   * example to create accounts prior to executing a method.
   */
  instructions?: TransactionInstruction[];

  /**
   * Commitment parameters to use for a transaction.
   */
  options?: ConfirmOptions;

  /**
   * @hidden
   *
   * Private namespace for development.
   */
  __private?: { logAccounts: boolean };
};

/**
 * A set of accounts mapping one-to-one to the program's accounts struct, i.e.,
 * the type deriving `#[derive(Accounts)]`.
 *
 * The name of each field should match the name for that account in the IDL.
 *
 * If multiple accounts are nested in the rust program, then they should be
 * nested here.
 */
export type Accounts<A extends IdlAccountItem[] = IdlAccountItem[]> = {
  [K in A[number]["name"]]: Account<A[number]>;
};

type Account<A extends IdlAccountItem> = A extends IdlAccounts
  ? Accounts<NonNullable<A["accounts"]>>
  : Address;

export function splitArgsAndCtx(
  idlIx: IdlInstruction,
  args: any[]
): [any[], Context] {
  let options = {};

  const inputLen = idlIx.args ? idlIx.args.length : 0;
  if (args.length > inputLen) {
    if (args.length !== inputLen + 1) {
      throw new Error("provided too many arguments ${args}");
    }
    options = args.pop();
  }

  return [args, options];
}
