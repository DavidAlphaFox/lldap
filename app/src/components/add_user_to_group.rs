use crate::{
    components::{
        select::{Select, SelectOption, SelectOptionProps},
        user_details::Group,
    },
    infra::common_component::{CommonComponent, CommonComponentParts},
};
use anyhow::{Error, Result};
use graphql_client::GraphQLQuery;
use std::collections::HashSet;
use yew::prelude::*;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../schema.graphql",
    query_path = "queries/add_user_to_group.graphql",
    response_derives = "Debug",
    variables_derives = "Clone",
    custom_scalars_module = "crate::infra::graphql"
)]
pub struct AddUserToGroup;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../schema.graphql",
    query_path = "queries/get_group_list.graphql",
    response_derives = "Debug",
    variables_derives = "Clone",
    custom_scalars_module = "crate::infra::graphql"
)]
pub struct GetGroupList;
type GroupListGroup = get_group_list::GetGroupListGroups;

impl From<GroupListGroup> for Group {
    fn from(group: GroupListGroup) -> Self {
        Self {
            id: group.id,
            display_name: group.display_name,
        }
    }
}

pub struct AddUserToGroupComponent {
    common: CommonComponentParts<Self>,
    /// The list of existing groups, initially not loaded.
    group_list: Option<Vec<Group>>,
    /// The currently selected group.
    selected_group: Option<Group>,
}

pub enum Msg {
    GroupListResponse(Result<get_group_list::ResponseData>),
    SubmitAddGroup,
    AddGroupResponse(Result<add_user_to_group::ResponseData>),
    SelectionChanged(Option<SelectOptionProps>),
}

#[derive(yew::Properties, Clone, PartialEq)]
pub struct Props {
    pub username: String,
    pub groups: Vec<Group>,
    pub on_user_added_to_group: Callback<Group>,
    pub on_error: Callback<Error>,
}

impl CommonComponent<AddUserToGroupComponent> for AddUserToGroupComponent {
    fn handle_msg(&mut self, msg: <Self as Component>::Message) -> Result<bool> {
        match msg {
            Msg::GroupListResponse(response) => {
                self.group_list = Some(response?.groups.into_iter().map(Into::into).collect());
                self.common.cancel_task();
            }
            Msg::SubmitAddGroup => return self.submit_add_group(),
            Msg::AddGroupResponse(response) => {
                response?;
                self.common.cancel_task();
                // Adding the user to the group succeeded, we're not in the process of adding a
                // group anymore.
                let group = self
                    .selected_group
                    .as_ref()
                    .expect("Could not get selected group")
                    .clone();
                // Remove the group from the dropdown.
                self.common.on_user_added_to_group.emit(group);
            }
            Msg::SelectionChanged(option_props) => {
                let was_some = self.selected_group.is_some();
                self.selected_group = option_props.map(|props| Group {
                    id: props.value.parse::<i64>().unwrap(),
                    display_name: props.text,
                });
                return Ok(self.selected_group.is_some() != was_some);
            }
        }
        Ok(true)
    }

    fn mut_common(&mut self) -> &mut CommonComponentParts<Self> {
        &mut self.common
    }
}

impl AddUserToGroupComponent {
    fn get_group_list(&mut self) {
        self.common.call_graphql::<GetGroupList, _>(
            get_group_list::Variables,
            Msg::GroupListResponse,
            "Error trying to fetch group list",
        );
    }

    fn submit_add_group(&mut self) -> Result<bool> {
        let group_id = match &self.selected_group {
            None => return Ok(false),
            Some(group) => group.id,
        };
        self.common.call_graphql::<AddUserToGroup, _>(
            add_user_to_group::Variables {
                user: self.common.username.clone(),
                group: group_id,
            },
            Msg::AddGroupResponse,
            "Error trying to initiate adding the user to a group",
        );
        Ok(true)
    }

    fn get_selectable_group_list(&self, group_list: &[Group]) -> Vec<Group> {
        let user_groups = self.common.groups.iter().collect::<HashSet<_>>();
        group_list
            .iter()
            .filter(|g| !user_groups.contains(g))
            .map(Clone::clone)
            .collect()
    }
}

impl Component for AddUserToGroupComponent {
    type Message = Msg;
    type Properties = Props;
    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut res = Self {
            common: CommonComponentParts::<Self>::create(props, link),
            group_list: None,
            selected_group: None,
        };
        res.get_group_list();
        res
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        CommonComponentParts::<Self>::update_and_report_error(
            self,
            msg,
            self.common.on_error.clone(),
        )
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.common.change(props)
    }

    fn view(&self) -> Html {
        if let Some(group_list) = &self.group_list {
            let to_add_group_list = self.get_selectable_group_list(group_list);
            #[allow(unused_braces)]
            let make_select_option = |group: Group| {
                html_nested! {
                    <SelectOption value=group.id.to_string() text=group.display_name key=group.id />
                }
            };
            html! {
            <div class="row">
              <div class="col-sm-3">
                <Select on_selection_change=self.common.callback(Msg::SelectionChanged)>
                  {
                    to_add_group_list
                        .into_iter()
                        .map(make_select_option)
                        .collect::<Vec<_>>()
                  }
                </Select>
              </div>
              <div class="col-sm-3">
                <button
                  class="btn btn-secondary"
                  disabled=self.selected_group.is_none() || self.common.is_task_running()
                  onclick=self.common.callback(|_| Msg::SubmitAddGroup)>
                  <i class="bi-person-plus me-2"></i>
                  {"Add to group"}
                </button>
              </div>
            </div>
            }
        } else {
            html! {
              {"Loading groups"}
            }
        }
    }
}
