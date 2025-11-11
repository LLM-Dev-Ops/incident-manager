use crate::error::{AppError, Result};
use crate::models::Incident;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Execution context holds runtime state and variables during playbook execution
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Incident being processed
    incident: Incident,

    /// Runtime variables
    variables: HashMap<String, JsonValue>,

    /// Step outputs for reference
    step_outputs: HashMap<String, HashMap<String, JsonValue>>,
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new(incident: Incident) -> Self {
        let mut variables = HashMap::new();

        // Initialize with incident variables
        variables.insert("incident_id".to_string(), JsonValue::String(incident.id.to_string()));
        variables.insert("incident_title".to_string(), JsonValue::String(incident.title.clone()));
        variables.insert("incident_severity".to_string(), JsonValue::String(format!("{:?}", incident.severity)));
        variables.insert("incident_type".to_string(), JsonValue::String(format!("{:?}", incident.incident_type)));
        variables.insert("incident_source".to_string(), JsonValue::String(incident.source.clone()));
        variables.insert("incident_state".to_string(), JsonValue::String(format!("{:?}", incident.state)));

        Self {
            incident,
            variables,
            step_outputs: HashMap::new(),
        }
    }

    /// Get the incident
    pub fn incident(&self) -> &Incident {
        &self.incident
    }

    /// Get a variable value
    pub fn get_variable(&self, key: &str) -> Option<&JsonValue> {
        self.variables.get(key)
    }

    /// Set a variable value
    pub fn set_variable(&mut self, key: String, value: JsonValue) {
        self.variables.insert(key, value);
    }

    /// Get all variables
    pub fn variables(&self) -> &HashMap<String, JsonValue> {
        &self.variables
    }

    /// Get step output
    pub fn get_step_output(&self, step_id: &str) -> Option<&HashMap<String, JsonValue>> {
        self.step_outputs.get(step_id)
    }

    /// Set step output
    pub fn set_step_output(&mut self, step_id: String, output: HashMap<String, JsonValue>) {
        self.step_outputs.insert(step_id, output);
    }

    /// Substitute variables in a string
    /// Supports {{variable_name}} syntax
    pub fn substitute_string(&self, template: &str) -> String {
        let mut result = template.to_string();

        // Find and replace all {{variable}} patterns
        for (key, value) in &self.variables {
            let placeholder = format!("{{{{{}}}}}", key);
            let replacement = match value {
                JsonValue::String(s) => s.clone(),
                JsonValue::Number(n) => n.to_string(),
                JsonValue::Bool(b) => b.to_string(),
                JsonValue::Null => "null".to_string(),
                _ => serde_json::to_string(value).unwrap_or_else(|_| "".to_string()),
            };
            result = result.replace(&placeholder, &replacement);
        }

        result
    }

    /// Substitute variables in JSON parameters
    pub fn substitute_parameters(&self, params: &HashMap<String, JsonValue>) -> HashMap<String, JsonValue> {
        let mut result = HashMap::new();

        for (key, value) in params {
            let substituted = self.substitute_json_value(value);
            result.insert(key.clone(), substituted);
        }

        result
    }

    /// Substitute variables in a JSON value
    fn substitute_json_value(&self, value: &JsonValue) -> JsonValue {
        match value {
            JsonValue::String(s) => JsonValue::String(self.substitute_string(s)),
            JsonValue::Object(map) => {
                let mut new_map = serde_json::Map::new();
                for (k, v) in map {
                    new_map.insert(k.clone(), self.substitute_json_value(v));
                }
                JsonValue::Object(new_map)
            }
            JsonValue::Array(arr) => {
                let new_arr: Vec<JsonValue> = arr.iter().map(|v| self.substitute_json_value(v)).collect();
                JsonValue::Array(new_arr)
            }
            _ => value.clone(),
        }
    }

    /// Evaluate a simple condition expression
    /// Supports: variable == value, variable != value, variable > value, variable < value
    pub fn evaluate_condition(&self, condition: &str) -> Result<bool> {
        let condition = condition.trim();

        if condition.is_empty() {
            return Ok(true);
        }

        // Parse condition
        if let Some((left, right)) = condition.split_once("==") {
            let left_val = self.resolve_value(left.trim())?;
            let right_val = self.resolve_value(right.trim())?;
            return Ok(left_val == right_val);
        }

        if let Some((left, right)) = condition.split_once("!=") {
            let left_val = self.resolve_value(left.trim())?;
            let right_val = self.resolve_value(right.trim())?;
            return Ok(left_val != right_val);
        }

        if let Some((left, right)) = condition.split_once(">=") {
            return self.compare_numeric(left.trim(), right.trim(), |a, b| a >= b);
        }

        if let Some((left, right)) = condition.split_once("<=") {
            return self.compare_numeric(left.trim(), right.trim(), |a, b| a <= b);
        }

        if let Some((left, right)) = condition.split_once(">") {
            return self.compare_numeric(left.trim(), right.trim(), |a, b| a > b);
        }

        if let Some((left, right)) = condition.split_once("<") {
            return self.compare_numeric(left.trim(), right.trim(), |a, b| a < b);
        }

        // If no operator, treat as boolean variable
        let value = self.resolve_value(condition)?;
        match value {
            JsonValue::Bool(b) => Ok(b),
            JsonValue::String(s) => Ok(s == "true"),
            _ => Ok(false),
        }
    }

    /// Resolve a value (variable or literal)
    fn resolve_value(&self, expr: &str) -> Result<JsonValue> {
        let expr = expr.trim();

        // Check if it's a variable reference
        if let Some(var_name) = expr.strip_prefix('$') {
            if let Some(value) = self.get_variable(var_name) {
                return Ok(value.clone());
            }
            return Err(AppError::Validation(format!("Variable '{}' not found", var_name)));
        }

        // Try to parse as JSON
        if let Ok(value) = serde_json::from_str(expr) {
            return Ok(value);
        }

        // Treat as string literal
        Ok(JsonValue::String(expr.to_string()))
    }

    /// Compare numeric values
    fn compare_numeric<F>(&self, left: &str, right: &str, op: F) -> Result<bool>
    where
        F: Fn(f64, f64) -> bool,
    {
        let left_val = self.resolve_value(left)?;
        let right_val = self.resolve_value(right)?;

        let left_num = match &left_val {
            JsonValue::Number(n) => n.as_f64().ok_or_else(|| {
                AppError::Validation(format!("Cannot convert '{}' to number", left))
            })?,
            JsonValue::String(s) => s.parse::<f64>().map_err(|_| {
                AppError::Validation(format!("Cannot parse '{}' as number", s))
            })?,
            _ => return Err(AppError::Validation(format!("'{}' is not a number", left))),
        };

        let right_num = match &right_val {
            JsonValue::Number(n) => n.as_f64().ok_or_else(|| {
                AppError::Validation(format!("Cannot convert '{}' to number", right))
            })?,
            JsonValue::String(s) => s.parse::<f64>().map_err(|_| {
                AppError::Validation(format!("Cannot parse '{}' as number", s))
            })?,
            _ => return Err(AppError::Validation(format!("'{}' is not a number", right))),
        };

        Ok(op(left_num, right_num))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{IncidentType, Severity};

    fn create_test_incident() -> Incident {
        Incident::new(
            "test-source".to_string(),
            "Test Incident".to_string(),
            "Test description".to_string(),
            Severity::P1,
            IncidentType::Infrastructure,
        )
    }

    #[test]
    fn test_context_initialization() {
        let incident = create_test_incident();
        let context = ExecutionContext::new(incident.clone());

        assert_eq!(
            context.get_variable("incident_id").unwrap().as_str().unwrap(),
            incident.id.to_string()
        );
        assert_eq!(
            context.get_variable("incident_title").unwrap().as_str().unwrap(),
            "Test Incident"
        );
        assert_eq!(
            context.get_variable("incident_severity").unwrap().as_str().unwrap(),
            "P1"
        );
    }

    #[test]
    fn test_variable_substitution() {
        let incident = create_test_incident();
        let mut context = ExecutionContext::new(incident);

        context.set_variable("custom_var".to_string(), JsonValue::String("test_value".to_string()));

        let result = context.substitute_string("Incident {{incident_title}} has {{custom_var}}");
        assert_eq!(result, "Incident Test Incident has test_value");
    }

    #[test]
    fn test_condition_evaluation() {
        let incident = create_test_incident();
        let mut context = ExecutionContext::new(incident);

        context.set_variable("count".to_string(), JsonValue::Number(serde_json::Number::from(5)));
        context.set_variable("name".to_string(), JsonValue::String("test".to_string()));

        assert!(context.evaluate_condition("$count == 5").unwrap());
        assert!(context.evaluate_condition("$count != 10").unwrap());
        assert!(context.evaluate_condition("$count > 3").unwrap());
        assert!(context.evaluate_condition("$count < 10").unwrap());
        assert!(context.evaluate_condition("$count >= 5").unwrap());
        assert!(context.evaluate_condition("$name == \"test\"").unwrap());
        assert!(!context.evaluate_condition("$count == 10").unwrap());
    }

    #[test]
    fn test_parameter_substitution() {
        let incident = create_test_incident();
        let mut context = ExecutionContext::new(incident);

        context.set_variable("service".to_string(), JsonValue::String("api-gateway".to_string()));

        let mut params = HashMap::new();
        params.insert(
            "message".to_string(),
            JsonValue::String("Incident {{incident_title}} affecting {{service}}".to_string()),
        );

        let result = context.substitute_parameters(&params);
        let message = result.get("message").unwrap().as_str().unwrap();
        assert!(message.contains("Test Incident"));
        assert!(message.contains("api-gateway"));
    }

    #[test]
    fn test_step_output() {
        let incident = create_test_incident();
        let mut context = ExecutionContext::new(incident);

        let mut output = HashMap::new();
        output.insert("result".to_string(), JsonValue::String("success".to_string()));

        context.set_step_output("step1".to_string(), output);

        let retrieved = context.get_step_output("step1").unwrap();
        assert_eq!(retrieved.get("result").unwrap().as_str().unwrap(), "success");
    }
}
