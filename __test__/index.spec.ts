import test from 'ava'

import { addMenuGroup, plus100 } from '../index'

test('sync function from native code', (t) => {
  const fixture = 42
  t.is(plus100(fixture), fixture + 100)
})

test.skip('create menu group', (t) => {
  const result = addMenuGroup({
      id: 1,
      key: "menu_group_1",
      title: "菜单组1",
      icon: "#",
      route: "menu-group-1",
      seq: 1,
      parentId: 0
  }, {
      author: "cx",
      id: "202605192050"
  }, {
      liquibaseRootFileFullPath: "D:\\sources\\markdown-lang\\ide-plugins\\vscode\\generated-code\\server\\src\\main\\resources\\db\\changelog\\db.changelog-master.xml",
      liquibaseNewFileIncludePath: "db/changelog/system/sys_menu/202605192050_insert_menu_group_1.xml",
      liquibaseNewFileFullPath: "D:\\sources\\markdown-lang\\ide-plugins\\vscode\\generated-code\\server\\src\\main\\resources\\db\\changelog\\system\\sys_menu"
  });
  t.is(result.files.length, 2);
})
