import { deleteUserModel, insertUserModel, listUserModels, updateUserModel, type ModelFields } from '../infrastructure/database/models'

export type { ModelFields }
export const listModels = listUserModels
export const createModel = insertUserModel
export const updateModel = updateUserModel
export const deleteModel = deleteUserModel
