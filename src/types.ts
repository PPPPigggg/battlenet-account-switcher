export interface AccountInfo {
  Id: string;
  Remark: string;
  Username: string;
  LastUsed: string;
  GroupId: string;
  LoggedIn: boolean;
}

export interface GroupInfo {
  Id: string;
  Name: string;
  CreatedAt: string;
}
