/**
 * Test stub - re-exports from the real implementation.
 * This allows tests to use the actual implementation.
 */

export {
  policy,
  cant,
  canOnly,
  addContext,
  compile,
  reason,
  defineSignal,
  defineConstant,
  mustHalt,
  mustAsk,
  mustModify,
  mustBlock,
} from '../index';
