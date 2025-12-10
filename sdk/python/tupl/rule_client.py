"""
gRPC client for Data Plane rule management operations.

Provides methods to install, remove, and query rules via the Data Plane gRPC API.
"""

import time
from typing import Optional

import grpc

from .generated.rule_installation_pb2 import (
    InstallRulesRequest,
    InstallRulesResponse,
    RemoveAgentRulesRequest,
    RemoveAgentRulesResponse,
    GetRuleStatsRequest,
    GetRuleStatsResponse,
    RuleInstance,
    ParamValue,
    StringList,
)
from .generated.rule_installation_pb2_grpc import DataPlaneStub


class RuleClientError(Exception):
    """Error communicating with the Data Plane for rule operations."""

    def __init__(self, message: str, status_code: Optional[grpc.StatusCode] = None):
        super().__init__(message)
        self.status_code = status_code


def param_dict_to_proto(params: dict) -> dict:
    """
    Convert params dictionary to proto ParamValue map.

    Auto-detects types and creates appropriate ParamValue instances.

    Args:
        params: Dictionary mapping param names to values

    Returns:
        Dictionary mapping param names to ParamValue proto messages

    Examples:
        >>> param_dict_to_proto({"tool_id": "search", "max_len": 100})
        {"tool_id": ParamValue(string_value="search"), "max_len": ParamValue(int_value=100)}
    """
    proto_params = {}

    for key, value in params.items():
        if isinstance(value, dict):
            # Handle nested format from demo: {"string_list": {"values": [...]}}
            if "string_list" in value:
                proto_params[key] = ParamValue(
                    string_list=StringList(values=value["string_list"]["values"])
                )
            elif "string_value" in value:
                proto_params[key] = ParamValue(string_value=value["string_value"])
            elif "int_value" in value:
                proto_params[key] = ParamValue(int_value=value["int_value"])
            elif "float_value" in value:
                proto_params[key] = ParamValue(float_value=value["float_value"])
            elif "bool_value" in value:
                proto_params[key] = ParamValue(bool_value=value["bool_value"])
            else:
                raise ValueError(f"Unknown nested param format for key '{key}': {value}")
        elif isinstance(value, list):
            # Auto-detect list type
            if all(isinstance(v, str) for v in value):
                proto_params[key] = ParamValue(string_list=StringList(values=value))
            else:
                raise ValueError(f"Unsupported list type for key '{key}': {value}")
        elif isinstance(value, str):
            proto_params[key] = ParamValue(string_value=value)
        elif isinstance(value, bool):
            # Check bool before int (bool is subclass of int in Python)
            proto_params[key] = ParamValue(bool_value=value)
        elif isinstance(value, int):
            proto_params[key] = ParamValue(int_value=value)
        elif isinstance(value, float):
            proto_params[key] = ParamValue(float_value=value)
        else:
            raise ValueError(f"Unsupported param type for key '{key}': {type(value)}")

    return proto_params


def dict_to_proto_rule(rule_dict: dict) -> RuleInstance:
    """
    Convert dictionary rule definition to proto RuleInstance.

    Auto-generates created_at_ms if not provided.

    Args:
        rule_dict: Dictionary with rule fields (rule_id, family_id, layer, etc.)

    Returns:
        RuleInstance proto message

    Example:
        >>> rule = {
        ...     "rule_id": "tw-001",
        ...     "family_id": "ToolWhitelist",
        ...     "layer": "L4",
        ...     "agent_id": "demo-agent",
        ...     "priority": 100,
        ...     "enabled": True,
        ...     "params": {"allowed_tool_ids": ["search", "update"]}
        ... }
        >>> proto_rule = dict_to_proto_rule(rule)
    """
    # Auto-generate timestamp if not provided
    created_at_ms = rule_dict.get("created_at_ms", int(time.time() * 1000))

    # Convert params to proto map
    params = rule_dict.get("params", {})
    proto_params = param_dict_to_proto(params)

    # Create RuleInstance proto
    return RuleInstance(
        rule_id=rule_dict["rule_id"],
        family_id=rule_dict["family_id"],
        layer=rule_dict["layer"],
        agent_id=rule_dict["agent_id"],
        priority=rule_dict.get("priority", 100),
        enabled=rule_dict.get("enabled", True),
        created_at_ms=created_at_ms,
        params=proto_params,
    )


class RuleClient:
    """
    gRPC client for Data Plane rule management.

    Provides methods to install, remove, and query rules.

    Example:
        >>> with RuleClient("localhost:50051") as client:
        ...     response = client.install_rules(
        ...         agent_id="my-agent",
        ...         rules=[rule_dict1, rule_dict2]
        ...     )
        ...     print(f"Installed {response.rules_installed} rules")
    """

    def __init__(
        self,
        url: str = "localhost:50051",
        timeout: float = 10.0,
        insecure: bool = True,
    ):
        """
        Initialize RuleClient.

        Args:
            url: Data Plane gRPC server URL (default: localhost:50051)
            timeout: Request timeout in seconds (default: 10s for bulk operations)
            insecure: Use insecure channel (default: True for development)
        """
        self.url = url
        self.timeout = timeout

        # Create gRPC channel
        if insecure:
            self.channel = grpc.insecure_channel(url)
        else:
            credentials = grpc.ssl_channel_credentials()
            self.channel = grpc.secure_channel(url, credentials)

        self.stub = DataPlaneStub(self.channel)

    def install_rules(
        self,
        agent_id: str,
        rules: list[dict],
        config_id: str = "default",
        owner: str = "system",
    ) -> InstallRulesResponse:
        """
        Install rules to Data Plane.

        Args:
            agent_id: Agent identifier
            rules: List of rule dictionaries to install
            config_id: Configuration ID (default: "default")
            owner: Owner identifier (default: "system")

        Returns:
            InstallRulesResponse with installation stats

        Raises:
            RuleClientError: If installation fails
        """
        try:
            # Convert dict rules to proto RuleInstance
            proto_rules = [dict_to_proto_rule(rule) for rule in rules]

            # Create request
            request = InstallRulesRequest(
                agent_id=agent_id,
                rules=proto_rules,
                config_id=config_id,
                owner=owner,
            )

            # Call gRPC
            response = self.stub.InstallRules(request, timeout=self.timeout)

            if not response.success:
                raise RuleClientError(
                    f"Rule installation failed: {response.message}",
                )

            return response

        except grpc.RpcError as e:
            raise RuleClientError(
                f"gRPC error installing rules: {e.details()}",
                status_code=e.code(),
            ) from e
        except Exception as e:
            raise RuleClientError(
                f"Error installing rules: {str(e)}",
            ) from e

    def remove_agent_rules(self, agent_id: str) -> RemoveAgentRulesResponse:
        """
        Remove all rules for an agent.

        Args:
            agent_id: Agent identifier

        Returns:
            RemoveAgentRulesResponse with removal count

        Raises:
            RuleClientError: If removal fails
        """
        try:
            # Create request
            request = RemoveAgentRulesRequest(agent_id=agent_id)

            # Call gRPC
            response = self.stub.RemoveAgentRules(request, timeout=self.timeout)

            if not response.success:
                raise RuleClientError(
                    f"Rule removal failed: {response.message}",
                )

            return response

        except grpc.RpcError as e:
            raise RuleClientError(
                f"gRPC error removing rules: {e.details()}",
                status_code=e.code(),
            ) from e
        except Exception as e:
            raise RuleClientError(
                f"Error removing rules: {str(e)}",
            ) from e

    def get_rule_stats(self) -> GetRuleStatsResponse:
        """
        Get rule statistics from Data Plane.

        Returns:
            GetRuleStatsResponse with bridge statistics

        Raises:
            RuleClientError: If stats query fails
        """
        try:
            # Create request (empty)
            request = GetRuleStatsRequest()

            # Call gRPC
            response = self.stub.GetRuleStats(request, timeout=self.timeout)

            return response

        except grpc.RpcError as e:
            raise RuleClientError(
                f"gRPC error fetching stats: {e.details()}",
                status_code=e.code(),
            ) from e
        except Exception as e:
            raise RuleClientError(
                f"Error fetching stats: {str(e)}",
            ) from e

    def close(self):
        """Close gRPC channel."""
        if self.channel:
            self.channel.close()

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()
